//! Host-only procedural macros for `nts3` plugin crates.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitStr, Type, UnOp};

const MAX_PARAMETERS: usize = 8;
const PARAMETER_NAME_LEN: usize = 21;

#[proc_macro_derive(Nts3Parameters, attributes(parameter))]
pub fn derive_nts3_parameters(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match expand_parameters(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FieldKind {
    Plain,
    Smoothed,
}

#[derive(Default)]
struct RawParameter {
    index: Option<(usize, Span)>,
    name: Option<(String, Span)>,
    min: Option<(i64, Span)>,
    max: Option<(i64, Span)>,
    center: Option<(i64, Span)>,
    default: Option<(i64, Span)>,
    parameter_type: Option<(String, Span)>,
    decimal_places: Option<(u64, Span)>,
    fixed_fraction_bits: Option<(u64, Span)>,
    smoothing_ms: Option<(f64, Span)>,
    assign: Option<(String, Span)>,
    curve: Option<(String, Span)>,
    curve_polarity: Option<(String, Span)>,
    mapping_min: Option<(i64, Span)>,
    mapping_max: Option<(i64, Span)>,
    mapping_default: Option<(i64, Span)>,
}

struct ParameterSpec {
    field: syn::Ident,
    field_span: Span,
    kind: FieldKind,
    index: usize,
    name: [u8; PARAMETER_NAME_LEN + 1],
    min: i16,
    max: i16,
    center: i16,
    default: i16,
    parameter_type: u8,
    format: u8,
    smoothing_ms: Option<f32>,
    assign: u8,
    curve: u8,
    mapping_min: i16,
    mapping_max: i16,
    mapping_default: i16,
}

fn expand_parameters(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new(
            input.generics.span(),
            "Nts3Parameters does not support generic parameter structs",
        ));
    }

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(syn::Error::new(
                    input.ident.span(),
                    "Nts3Parameters requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                input.ident.span(),
                "Nts3Parameters can only be derived for structs",
            ));
        }
    };

    if fields.len() > MAX_PARAMETERS {
        return Err(syn::Error::new(
            input.ident.span(),
            format!(
                "NTS-3 supports at most {MAX_PARAMETERS} parameters; found {}",
                fields.len()
            ),
        ));
    }

    let mut specs = Vec::with_capacity(fields.len());
    for (declaration_index, field) in fields.into_iter().enumerate() {
        let field_span = field.span();
        let field_ident = field.ident.ok_or_else(|| {
            syn::Error::new(field_span, "Nts3Parameters requires named parameter fields")
        })?;
        let kind = field_kind(&field.ty)?;
        let parameter_attributes: Vec<_> = field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("parameter"))
            .collect();
        if parameter_attributes.len() != 1 {
            return Err(syn::Error::new(
                field_span,
                "each parameter field requires exactly one #[parameter(...)] attribute",
            ));
        }
        let raw = parse_parameter(parameter_attributes[0])?;
        specs.push(validate_parameter(
            field_ident,
            field_span,
            kind,
            declaration_index,
            raw,
        )?);
    }

    validate_indices(&specs)?;
    specs.sort_by_key(|spec| spec.index);

    let descriptors = specs.iter().map(descriptor_tokens);
    let descriptor_padding =
        (specs.len()..MAX_PARAMETERS).map(|_| quote!(::nts3::__private::UNUSED_PARAM));
    let mappings = specs.iter().map(mapping_tokens);
    let mapping_padding =
        (specs.len()..MAX_PARAMETERS).map(|_| quote!(::nts3::__private::UNUSED_MAPPING));

    let defaults = specs.iter().map(|spec| {
        let field = &spec.field;
        let index = spec.index;
        match spec.kind {
            FieldKind::Plain => quote! {
                #field: ::nts3::Parameter::from_descriptor(
                    &<Self as ::nts3::Nts3Parameters>::DESCRIPTORS[#index]
                )
            },
            FieldKind::Smoothed => {
                let smoothing_ms = spec.smoothing_ms.expect("validated smoothed parameter");
                quote! {
                    #field: ::nts3::SmoothedParameter::from_descriptor(
                        &<Self as ::nts3::Nts3Parameters>::DESCRIPTORS[#index],
                        #smoothing_ms
                    )
                }
            }
        }
    });

    let get_arms = specs.iter().map(|spec| {
        let index = spec.index as u8;
        let field = &spec.field;
        quote!(#index => Some(i32::from(self.#field.raw())),)
    });
    let set_arms = specs.iter().map(|spec| {
        let index = spec.index as u8;
        let field = &spec.field;
        quote!(#index => { self.#field.set(value); true },)
    });

    let smoothed: Vec<_> = specs
        .iter()
        .filter(|spec| spec.kind == FieldKind::Smoothed)
        .map(|spec| &spec.field)
        .collect();
    let initialize_smoothers = smoothed
        .iter()
        .map(|field| quote!(self.#field.set_sample_rate(sample_rate);));
    let reset_smoothers = smoothed
        .iter()
        .map(|field| quote!(self.#field.reset_smoothing();));
    let begin_block = smoothed
        .iter()
        .map(|field| quote!(self.#field.begin_block();));
    let end_block = smoothed
        .iter()
        .map(|field| quote!(self.#field.end_block();));

    let ident = input.ident;
    let count = specs.len();
    Ok(quote! {
        #[automatically_derived]
        impl ::core::default::Default for #ident {
            fn default() -> Self {
                Self { #(#defaults,)* }
            }
        }

        #[automatically_derived]
        impl ::nts3::__private::Sealed for #ident {}

        #[automatically_derived]
        impl ::nts3::Nts3Parameters for #ident {
            const DESCRIPTORS: [::nts3::__private::UnitParam; 8] = [
                #(#descriptors,)*
                #(#descriptor_padding,)*
            ];
            const MAPPINGS: [::nts3::__private::GenericfxParamMapping; 8] = [
                #(#mappings,)*
                #(#mapping_padding,)*
            ];
            const COUNT: usize = #count;

            fn get(&self, index: u8) -> Option<i32> {
                match index {
                    #(#get_arms)*
                    _ => None,
                }
            }

            fn set(&mut self, index: u8, value: i32) -> bool {
                match index {
                    #(#set_arms)*
                    _ => false,
                }
            }

            fn initialize_smoothers(&mut self, sample_rate: f32) {
                #(#initialize_smoothers)*
            }

            fn reset_smoothers(&mut self) {
                #(#reset_smoothers)*
            }

            fn begin_block(&mut self) {
                #(#begin_block)*
            }

            fn end_block(&mut self) {
                #(#end_block)*
            }
        }
    })
}

fn field_kind(ty: &Type) -> syn::Result<FieldKind> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new(
            ty.span(),
            "parameter fields must have type Parameter or SmoothedParameter",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new(
            ty.span(),
            "parameter fields must have type Parameter or SmoothedParameter",
        ));
    };
    if !segment.arguments.is_empty() {
        return Err(syn::Error::new(
            ty.span(),
            "parameter fields must have type Parameter or SmoothedParameter",
        ));
    }
    match segment.ident.to_string().as_str() {
        "Parameter" => Ok(FieldKind::Plain),
        "SmoothedParameter" => Ok(FieldKind::Smoothed),
        _ => Err(syn::Error::new(
            ty.span(),
            "unsupported parameter field type; expected Parameter or SmoothedParameter",
        )),
    }
}

fn parse_parameter(attribute: &syn::Attribute) -> syn::Result<RawParameter> {
    let mut raw = RawParameter::default();
    attribute.parse_nested_meta(|meta| {
        let Some(key) = meta.path.get_ident().map(ToString::to_string) else {
            return Err(meta.error("parameter option must be a simple identifier"));
        };
        match key.as_str() {
            "name" | "parameter_type" | "assign" | "curve" | "curve_polarity" => {
                let value: LitStr = meta.value()?.parse()?;
                let item = (value.value(), value.span());
                match key.as_str() {
                    "name" => set_once(&mut raw.name, item, value.span(), "name"),
                    "parameter_type" => set_once(
                        &mut raw.parameter_type,
                        item,
                        value.span(),
                        "parameter_type",
                    ),
                    "assign" => set_once(&mut raw.assign, item, value.span(), "assign"),
                    "curve" => set_once(&mut raw.curve, item, value.span(), "curve"),
                    "curve_polarity" => set_once(
                        &mut raw.curve_polarity,
                        item,
                        value.span(),
                        "curve_polarity",
                    ),
                    _ => unreachable!(),
                }
            }
            "index" | "decimal_places" | "fixed_fraction_bits" => {
                let expression: Expr = meta.value()?.parse()?;
                let value = parse_unsigned_integer(&expression)?;
                let item = (value, expression.span());
                match key.as_str() {
                    "index" => {
                        let index = usize::try_from(value).map_err(|_| {
                            syn::Error::new(expression.span(), "parameter index is too large")
                        })?;
                        set_once(
                            &mut raw.index,
                            (index, expression.span()),
                            expression.span(),
                            "index",
                        )
                    }
                    "decimal_places" => set_once(
                        &mut raw.decimal_places,
                        item,
                        expression.span(),
                        "decimal_places",
                    ),
                    "fixed_fraction_bits" => set_once(
                        &mut raw.fixed_fraction_bits,
                        item,
                        expression.span(),
                        "fixed_fraction_bits",
                    ),
                    _ => unreachable!(),
                }
            }
            "min" | "max" | "center" | "default" | "mapping_min" | "mapping_max"
            | "mapping_default" => {
                let expression: Expr = meta.value()?.parse()?;
                let value = parse_signed_integer(&expression)?;
                let item = (value, expression.span());
                match key.as_str() {
                    "min" => set_once(&mut raw.min, item, expression.span(), "min"),
                    "max" => set_once(&mut raw.max, item, expression.span(), "max"),
                    "center" => set_once(&mut raw.center, item, expression.span(), "center"),
                    "default" => set_once(&mut raw.default, item, expression.span(), "default"),
                    "mapping_min" => {
                        set_once(&mut raw.mapping_min, item, expression.span(), "mapping_min")
                    }
                    "mapping_max" => {
                        set_once(&mut raw.mapping_max, item, expression.span(), "mapping_max")
                    }
                    "mapping_default" => set_once(
                        &mut raw.mapping_default,
                        item,
                        expression.span(),
                        "mapping_default",
                    ),
                    _ => unreachable!(),
                }
            }
            "smoothing_ms" => {
                let expression: Expr = meta.value()?.parse()?;
                let value = parse_number(&expression)?;
                set_once(
                    &mut raw.smoothing_ms,
                    (value, expression.span()),
                    expression.span(),
                    "smoothing_ms",
                )
            }
            _ => Err(meta.error(format!("unknown parameter option `{key}`"))),
        }
    })?;
    Ok(raw)
}

fn set_once<T>(destination: &mut Option<T>, value: T, span: Span, name: &str) -> syn::Result<()> {
    if destination.is_some() {
        Err(syn::Error::new(
            span,
            format!("duplicate parameter option `{name}`"),
        ))
    } else {
        *destination = Some(value);
        Ok(())
    }
}

fn parse_signed_integer(expression: &Expr) -> syn::Result<i64> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => value.base10_parse(),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => parse_signed_integer(expr)?.checked_neg().ok_or_else(|| {
            syn::Error::new(
                expression.span(),
                "integer literal is out of supported range",
            )
        }),
        _ => Err(syn::Error::new(
            expression.span(),
            "expected an integer literal",
        )),
    }
}

fn parse_unsigned_integer(expression: &Expr) -> syn::Result<u64> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => value.base10_parse(),
        _ => Err(syn::Error::new(
            expression.span(),
            "expected a nonnegative integer literal",
        )),
    }
}

fn parse_number(expression: &Expr) -> syn::Result<f64> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Float(value),
            ..
        }) => value.base10_parse(),
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => value.base10_parse(),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => Ok(-parse_number(expr)?),
        _ => Err(syn::Error::new(
            expression.span(),
            "expected a numeric literal",
        )),
    }
}

fn validate_parameter(
    field: syn::Ident,
    field_span: Span,
    kind: FieldKind,
    declaration_index: usize,
    raw: RawParameter,
) -> syn::Result<ParameterSpec> {
    let index = raw.index.map_or(declaration_index, |value| value.0);
    let index_span = raw.index.map_or(field_span, |value| value.1);
    if index >= MAX_PARAMETERS {
        return Err(syn::Error::new(
            index_span,
            "parameter index must be in 0..=7",
        ));
    }

    let (name_value, name_span) = required(raw.name, field_span, "name")?;
    let name = encode_name(&name_value, name_span)?;
    let min = checked_i16(required(raw.min, field_span, "min")?, "min")?;
    let max = checked_i16(required(raw.max, field_span, "max")?, "max")?;
    if min.0 > max.0 {
        return Err(syn::Error::new(
            min.1,
            "parameter range is invalid: `min` must be less than or equal to `max`",
        ));
    }
    let default = checked_i16(required(raw.default, field_span, "default")?, "default")?;
    ensure_in_descriptor(default, min.0, max.0, "default")?;
    let center = match raw.center {
        Some(value) => checked_i16(value, "center")?,
        None => min,
    };
    ensure_in_descriptor(center, min.0, max.0, "center")?;

    let (parameter_type_name, parameter_type_span) =
        required(raw.parameter_type, field_span, "parameter_type")?;
    let parameter_type = display_type(&parameter_type_name).ok_or_else(|| {
        syn::Error::new(
            parameter_type_span,
            format!("unsupported parameter_type `{parameter_type_name}`"),
        )
    })?;

    if raw.decimal_places.is_some() && raw.fixed_fraction_bits.is_some() {
        return Err(syn::Error::new(
            raw.fixed_fraction_bits.expect("present").1,
            "`decimal_places` and `fixed_fraction_bits` are mutually exclusive",
        ));
    }
    let format = if let Some((places, span)) = raw.decimal_places {
        if places > 15 {
            return Err(syn::Error::new(span, "decimal_places must be in 0..=15"));
        }
        places as u8 | 0x10
    } else if let Some((bits, span)) = raw.fixed_fraction_bits {
        if bits > 15 {
            return Err(syn::Error::new(
                span,
                "fixed_fraction_bits must be in 0..=15",
            ));
        }
        bits as u8
    } else {
        0
    };

    let smoothing_ms = match (kind, raw.smoothing_ms) {
        (FieldKind::Plain, Some((_, span))) => {
            return Err(syn::Error::new(
                span,
                "smoothing_ms is only valid for SmoothedParameter fields",
            ));
        }
        (FieldKind::Plain, None) => None,
        (FieldKind::Smoothed, None) => {
            return Err(syn::Error::new(
                field_span,
                "SmoothedParameter fields require `smoothing_ms`",
            ));
        }
        (FieldKind::Smoothed, Some((value, span))) => {
            if !value.is_finite() || value <= 0.0 || value > f32::MAX as f64 {
                return Err(syn::Error::new(
                    span,
                    "smoothing_ms must be a finite number greater than zero",
                ));
            }
            Some(value as f32)
        }
    };

    let (assign_name, assign_span) = raw
        .assign
        .unwrap_or_else(|| (String::from("none"), field_span));
    let assign = assignment(&assign_name).ok_or_else(|| {
        syn::Error::new(assign_span, format!("unsupported assign `{assign_name}`"))
    })?;
    let (curve_name, curve_span) = raw
        .curve
        .unwrap_or_else(|| (String::from("linear"), field_span));
    let curve_value = curve(&curve_name)
        .ok_or_else(|| syn::Error::new(curve_span, format!("unsupported curve `{curve_name}`")))?;
    let (polarity_name, polarity_span) = raw
        .curve_polarity
        .unwrap_or_else(|| (String::from("unipolar"), field_span));
    let polarity = curve_polarity(&polarity_name).ok_or_else(|| {
        syn::Error::new(
            polarity_span,
            format!("unsupported curve_polarity `{polarity_name}`"),
        )
    })?;
    let curve = curve_value | (polarity << 7);

    let mapping_min = match raw.mapping_min {
        Some(value) => checked_i16(value, "mapping_min")?,
        None => min,
    };
    let mapping_max = match raw.mapping_max {
        Some(value) => checked_i16(value, "mapping_max")?,
        None => max,
    };
    let mapping_default = match raw.mapping_default {
        Some(value) => checked_i16(value, "mapping_default")?,
        None => default,
    };
    ensure_in_descriptor(mapping_min, min.0, max.0, "mapping_min")?;
    ensure_in_descriptor(mapping_max, min.0, max.0, "mapping_max")?;
    ensure_in_descriptor(mapping_default, min.0, max.0, "mapping_default")?;

    Ok(ParameterSpec {
        field,
        field_span,
        kind,
        index,
        name,
        min: min.0,
        max: max.0,
        center: center.0,
        default: default.0,
        parameter_type,
        format,
        smoothing_ms,
        assign,
        curve,
        mapping_min: mapping_min.0,
        mapping_max: mapping_max.0,
        mapping_default: mapping_default.0,
    })
}

fn required<T>(value: Option<T>, span: Span, name: &str) -> syn::Result<T> {
    value
        .ok_or_else(|| syn::Error::new(span, format!("missing required parameter option `{name}`")))
}

fn checked_i16(value: (i64, Span), name: &str) -> syn::Result<(i16, Span)> {
    i16::try_from(value.0)
        .map(|converted| (converted, value.1))
        .map_err(|_| {
            syn::Error::new(
                value.1,
                format!("`{name}` must fit the NTS-3 signed 16-bit value range"),
            )
        })
}

fn ensure_in_descriptor(value: (i16, Span), min: i16, max: i16, name: &str) -> syn::Result<()> {
    if value.0 < min || value.0 > max {
        Err(syn::Error::new(
            value.1,
            format!("`{name}` must be inside the parameter range {min}..={max}"),
        ))
    } else {
        Ok(())
    }
}

fn encode_name(value: &str, span: Span) -> syn::Result<[u8; PARAMETER_NAME_LEN + 1]> {
    if value.is_empty() {
        return Err(syn::Error::new(span, "parameter name must not be empty"));
    }
    if value.len() > PARAMETER_NAME_LEN {
        return Err(syn::Error::new(
            span,
            format!("parameter name must be at most {PARAMETER_NAME_LEN} characters"),
        ));
    }
    if !value.bytes().all(valid_sdk_character) {
        return Err(syn::Error::new(
            span,
            "parameter name contains an invalid character; use 7-bit letters, digits, space, `-`, or `_`",
        ));
    }
    let mut encoded = [0; PARAMETER_NAME_LEN + 1];
    encoded[..value.len()].copy_from_slice(value.as_bytes());
    Ok(encoded)
}

fn valid_sdk_character(value: u8) -> bool {
    value == b' ' || value == b'-' || value == b'_' || value.is_ascii_alphanumeric()
}

fn display_type(value: &str) -> Option<u8> {
    Some(match value {
        "none" => 0,
        "percent" => 1,
        "db" => 2,
        "cents" => 3,
        "semi" | "semitones" => 4,
        "oct" | "octaves" => 5,
        "hertz" => 6,
        "khertz" | "kilohertz" => 7,
        "bpm" => 8,
        "msec" | "milliseconds" => 9,
        "sec" | "seconds" => 10,
        "enum" => 11,
        // SDK custom strings require an explicit static string-table API. That
        // syntax is intentionally deferred rather than emitting null strings.
        "drywet" => 14,
        "pan" => 15,
        "spread" => 16,
        "onoff" => 17,
        "midi_note" => 18,
        _ => return None,
    })
}

fn assignment(value: &str) -> Option<u8> {
    Some(match value {
        "none" => 0,
        "x" => 1,
        "y" => 2,
        "depth" => 3,
        _ => return None,
    })
}

fn curve(value: &str) -> Option<u8> {
    Some(match value {
        "linear" => 0,
        "exp" => 1,
        "log" => 2,
        "toggle" => 3,
        "minclip" => 4,
        "maxclip" => 5,
        _ => return None,
    })
}

fn curve_polarity(value: &str) -> Option<u8> {
    Some(match value {
        "unipolar" => 0,
        "bipolar" => 1,
        _ => return None,
    })
}

fn validate_indices(specs: &[ParameterSpec]) -> syn::Result<()> {
    let mut occupied = [None; MAX_PARAMETERS];
    for spec in specs {
        if let Some(previous) = occupied[spec.index] {
            let mut error = syn::Error::new(
                spec.field_span,
                format!("duplicate parameter index {}", spec.index),
            );
            error.combine(syn::Error::new(previous, "first used here"));
            return Err(error);
        }
        occupied[spec.index] = Some(spec.field_span);
    }
    for (index, entry) in occupied.iter().enumerate().take(specs.len()) {
        if entry.is_none() {
            return Err(syn::Error::new(
                specs
                    .iter()
                    .find(|spec| spec.index >= specs.len())
                    .map_or(Span::call_site(), |spec| spec.field_span),
                format!("parameter indices must be contiguous from 0; missing index {index}"),
            ));
        }
    }
    Ok(())
}

fn descriptor_tokens(spec: &ParameterSpec) -> proc_macro2::TokenStream {
    let min = spec.min;
    let max = spec.max;
    let center = spec.center;
    let default = spec.default;
    let parameter_type = spec.parameter_type;
    let format = spec.format;
    let name = spec.name.iter();
    quote! {
        ::nts3::__private::UnitParam::new(
            #min,
            #max,
            #center,
            #default,
            #parameter_type,
            ::nts3::__private::UnitParamFormat::from_raw(#format),
            [#(#name,)*]
        )
    }
}

fn mapping_tokens(spec: &ParameterSpec) -> proc_macro2::TokenStream {
    let assign = spec.assign;
    let curve = spec.curve;
    let min = spec.mapping_min;
    let max = spec.mapping_max;
    let default = spec.mapping_default;
    quote! {
        ::nts3::__private::GenericfxParamMapping::new(
            #assign,
            ::nts3::__private::GenericfxCurve::from_raw(#curve),
            #min,
            #max,
            #default
        )
    }
}
