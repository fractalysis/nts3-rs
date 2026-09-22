//! Host-only procedural macros for `nts3` plugin crates.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, ItemImpl, Lit, LitStr, Meta,
    MetaNameValue, Token, Type, UnOp,
};

const MAX_PARAMETERS: usize = 8;
const PARAMETER_NAME_LEN: usize = 21;
const UNIT_NAME_LEN: usize = 19;
const PLATFORM_SDRAM_LIMIT: u64 = 3 * 1024 * 1024;
const PARAMETER_TYPE_DRYWET: u8 = 14;
const GENERICFX_ASSIGN_DEPTH: u8 = 3;
const GENERICFX_CURVE_EXP: u8 = 1;
const GENERICFX_CURVE_BIPOLAR: u8 = 1;

/// Generates the complete NTS-3 genericfx header and callback adapter for one
/// concrete `Nts3Plugin` implementation.
#[proc_macro_attribute]
pub fn plugin(arguments: TokenStream, input: TokenStream) -> TokenStream {
    let arguments = syn::parse_macro_input!(arguments as PluginArguments);
    let implementation = syn::parse_macro_input!(input as ItemImpl);
    match expand_plugin(arguments, implementation) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[derive(Default)]
struct PluginArguments {
    name: Option<(String, Span)>,
    developer_id: Option<(u32, Span)>,
    unit_id: Option<(u32, Span)>,
    sdram_bytes: Option<(u32, Span)>,
}

impl Parse for PluginArguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let metadata = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        let mut arguments = Self::default();
        for meta in metadata {
            let Meta::NameValue(MetaNameValue { path, value, .. }) = meta else {
                return Err(syn::Error::new(
                    meta.span(),
                    "plugin options must use `name = value` syntax",
                ));
            };
            let Some(key) = path.get_ident().map(ToString::to_string) else {
                return Err(syn::Error::new(
                    path.span(),
                    "plugin option must be an identifier",
                ));
            };
            match key.as_str() {
                "name" => {
                    let Expr::Lit(ExprLit {
                        lit: Lit::Str(value),
                        ..
                    }) = value
                    else {
                        return Err(syn::Error::new(
                            value.span(),
                            "plugin name must be a string literal",
                        ));
                    };
                    let span = value.span();
                    set_once(&mut arguments.name, (value.value(), span), span, "name")?;
                }
                "developer_id" | "unit_id" | "sdram_bytes" => {
                    let parsed = parse_unsigned_integer(&value)?;
                    let converted = u32::try_from(parsed).map_err(|_| {
                        syn::Error::new(
                            value.span(),
                            format!("`{key}` must fit an unsigned 32-bit integer"),
                        )
                    })?;
                    match key.as_str() {
                        "developer_id" => set_once(
                            &mut arguments.developer_id,
                            (converted, value.span()),
                            value.span(),
                            "developer_id",
                        )?,
                        "unit_id" => set_once(
                            &mut arguments.unit_id,
                            (converted, value.span()),
                            value.span(),
                            "unit_id",
                        )?,
                        "sdram_bytes" => set_once(
                            &mut arguments.sdram_bytes,
                            (converted, value.span()),
                            value.span(),
                            "sdram_bytes",
                        )?,
                        _ => unreachable!(),
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        path.span(),
                        format!("unknown plugin option `{key}`"),
                    ));
                }
            }
        }
        Ok(arguments)
    }
}

fn expand_plugin(
    arguments: PluginArguments,
    implementation: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    if !implementation.generics.params.is_empty() || implementation.generics.where_clause.is_some()
    {
        return Err(syn::Error::new(
            implementation.generics.span(),
            "#[nts3::plugin] requires a concrete, non-generic trait implementation",
        ));
    }
    let Some((negative, trait_path, _)) = &implementation.trait_ else {
        return Err(syn::Error::new(
            implementation.impl_token.span(),
            "#[nts3::plugin] must be applied to an impl of Nts3Plugin",
        ));
    };
    let implements_nts3_plugin = trait_path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Nts3Plugin");
    if negative.is_some() || !implements_nts3_plugin {
        return Err(syn::Error::new(
            trait_path.span(),
            "#[nts3::plugin] must be applied to an impl of Nts3Plugin",
        ));
    }

    let self_type = &implementation.self_ty;
    let metadata_span = implementation.impl_token.span();
    let (name, name_span) = required(arguments.name, metadata_span, "name")?;
    let encoded_name = encode_sdk_name(&name, name_span, UNIT_NAME_LEN, "unit")?;
    let (developer_id, developer_span) =
        required(arguments.developer_id, metadata_span, "developer_id")?;
    if developer_id == 0 || is_korg_id(developer_id) {
        return Err(syn::Error::new(
            developer_span,
            "developer_id is reserved; zero and every upper/lower-case spelling of KORG are forbidden",
        ));
    }
    let (unit_id, _) = required(arguments.unit_id, metadata_span, "unit_id")?;
    let (sdram_bytes, sdram_span) = required(arguments.sdram_bytes, metadata_span, "sdram_bytes")?;
    if sdram_bytes == 0 {
        return Err(syn::Error::new(
            sdram_span,
            "sdram_bytes must be greater than zero",
        ));
    }
    if u64::from(sdram_bytes) > PLATFORM_SDRAM_LIMIT {
        return Err(syn::Error::new(
            sdram_span,
            "sdram_bytes exceeds the NTS-3 3 MiB per-runtime limit",
        ));
    }

    let major = cargo_version_component("CARGO_PKG_VERSION_MAJOR", metadata_span)?;
    let minor = cargo_version_component("CARGO_PKG_VERSION_MINOR", metadata_span)?;
    let patch = cargo_version_component("CARGO_PKG_VERSION_PATCH", metadata_span)?;
    let packed_version = pack_version(major, minor, patch, metadata_span)?;
    let name_bytes = encoded_name.iter();

    Ok(quote! {
        #implementation

        // A macro_export item always occupies the invoking crate's root macro
        // namespace, even when this attribute is inside a module. A second
        // plugin attribute therefore receives a direct duplicate-definition
        // diagnostic before an artifact with ambiguous exports can be built.
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __nts3_one_exported_plugin_per_artifact {
            () => {
                #[global_allocator]
                static __NTS3_HOST_PROBE_ALLOCATOR: $crate::__Nts3HostProbeAllocator =
                    $crate::__Nts3HostProbeAllocator;
            };
        }

        ::nts3::__install_runtime_glue!();

        #[doc(hidden)]
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".unit_header")]
        pub static unit_header: ::nts3::__private::GenericfxUnitHeader =
            ::nts3::__private::GenericfxUnitHeader::new(
                ::nts3::__private::UnitHeader::new(
                    ::core::mem::size_of::<::nts3::__private::GenericfxUnitHeader>() as u32,
                    ::nts3::__private::UNIT_TARGET_NTS3_KAOSS_GENERICFX,
                    ::nts3::__private::UNIT_API_VERSION,
                    #developer_id,
                    #unit_id,
                    #packed_version,
                    [#(#name_bytes,)*],
                    <<#self_type as ::nts3::Nts3Plugin>::Parameters as ::nts3::Nts3Parameters>::COUNT as u32,
                    <<#self_type as ::nts3::Nts3Plugin>::Parameters as ::nts3::Nts3Parameters>::DESCRIPTORS,
                ),
                <<#self_type as ::nts3::Nts3Plugin>::Parameters as ::nts3::Nts3Parameters>::MAPPINGS,
            );

        #[doc(hidden)]
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".nts3_resources")]
        pub static nts3_resources: ::nts3::__private::ResourceRecord =
            ::nts3::__private::ResourceRecord::new(#sdram_bytes);

        /// Runs the framework's native, arena-isolated initialization and
        /// post-initialization allocation probe for this concrete plugin.
        #[cfg(not(target_os = "none"))]
        #[doc(hidden)]
        pub fn nts3_host_probe() -> Result<::nts3::host::HostProbeReport, &'static str> {
            ::nts3::host::probe::<#self_type>(#sdram_bytes)
        }

        #[cfg(not(target_os = "none"))]
        #[doc(hidden)]
        pub use ::nts3::FrameworkAllocator as __Nts3HostProbeAllocator;

        #[doc(hidden)]
        static __NTS3_RUNTIME: ::nts3::runtime::ExportRuntime<#self_type> =
            ::nts3::runtime::ExportRuntime::new();

        /// # Safety
        /// The descriptor must satisfy the Korg NTS-3 runtime ABI for this call.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn unit_init(
            descriptor: *const ::nts3::__private::UnitRuntimeDescriptor,
        ) -> i8 {
            // SAFETY: the foreign caller supplies the descriptor under the SDK
            // contract; the runtime validates all readable metadata and hooks.
            unsafe { __NTS3_RUNTIME.initialize(descriptor, #sdram_bytes) }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_teardown() { __NTS3_RUNTIME.teardown(); }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_reset() { __NTS3_RUNTIME.reset(); }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_resume() { __NTS3_RUNTIME.resume(); }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_suspend() { __NTS3_RUNTIME.suspend(); }

        /// # Safety
        /// For nonzero frames, pointers must cover interleaved stereo buffers
        /// and be disjoint or exactly equal as required by the SDK.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn unit_render(
            input: *const f32,
            output: *mut f32,
            frames: u32,
        ) {
            // SAFETY: the foreign caller owns the audio buffers under the SDK
            // contract; the runtime bounds their use to this callback.
            unsafe { __NTS3_RUNTIME.render(input, output, frames); }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_get_param_value(id: u8) -> i32 {
            __NTS3_RUNTIME.get_parameter(id)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_get_param_str_value(
            id: u8,
            value: i32,
        ) -> *const ::core::ffi::c_char {
            __NTS3_RUNTIME.parameter_string_value(id, value)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_set_param_value(id: u8, value: i32) {
            __NTS3_RUNTIME.set_parameter(id, value);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_set_tempo(tempo: u32) {
            __NTS3_RUNTIME.set_tempo(tempo);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_tempo_4ppqn_tick(counter: u32) {
            __NTS3_RUNTIME.tempo_4ppqn_tick(counter);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn unit_touch_event(id: u8, phase: u8, x: u32, y: u32) {
            __NTS3_RUNTIME.touch_event(id, phase, x, y);
        }
    })
}

fn cargo_version_component(name: &str, span: Span) -> syn::Result<u32> {
    let value = std::env::var(name).map_err(|_| {
        syn::Error::new(
            span,
            format!("Cargo did not provide `{name}` to #[nts3::plugin]"),
        )
    })?;
    value.parse().map_err(|_| {
        syn::Error::new(
            span,
            format!("Cargo package version component `{value}` is not an integer"),
        )
    })
}

fn pack_version(major: u32, minor: u32, patch: u32, span: Span) -> syn::Result<u32> {
    if major > 0x7f || minor > 0x7f || patch > 0x7f {
        return Err(syn::Error::new(
            span,
            "Cargo package version components must each fit the Korg 7-bit range 0..=127",
        ));
    }
    Ok((major << 16) | (minor << 8) | patch)
}

fn is_korg_id(identifier: u32) -> bool {
    identifier
        .to_be_bytes()
        .into_iter()
        .zip(*b"korg")
        .all(|(actual, expected)| actual.to_ascii_lowercase() == expected)
}

fn encode_sdk_name(value: &str, span: Span, maximum: usize, kind: &str) -> syn::Result<Vec<u8>> {
    if value.is_empty() {
        return Err(syn::Error::new(
            span,
            format!("{kind} name must not be empty"),
        ));
    }
    if value.len() > maximum {
        return Err(syn::Error::new(
            span,
            format!("{kind} name must be at most {maximum} characters"),
        ));
    }
    if !value.bytes().all(valid_sdk_character) {
        return Err(syn::Error::new(
            span,
            format!(
                "{kind} name contains an invalid character; use 7-bit letters, digits, space, `-`, or `_`"
            ),
        ));
    }
    let mut encoded = vec![0; maximum + 1];
    encoded[..value.len()].copy_from_slice(value.as_bytes());
    Ok(encoded)
}

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

    if raw.decimal_places.is_some()
        && let Some((_, span)) = raw.fixed_fraction_bits
    {
        return Err(syn::Error::new(
            span,
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
    if parameter_type == PARAMETER_TYPE_DRYWET
        && assign == GENERICFX_ASSIGN_DEPTH
        && (curve_value != GENERICFX_CURVE_EXP || polarity != GENERICFX_CURVE_BIPOLAR)
    {
        return Err(syn::Error::new(
            assign_span,
            "a `drywet` parameter assigned to `depth` requires `curve = \"exp\"` and `curve_polarity = \"bipolar\"`",
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn korg_case_variants_and_zero_are_reserved() {
        for first in *b"Kk" {
            for second in *b"Oo" {
                for third in *b"Rr" {
                    for fourth in *b"Gg" {
                        assert!(is_korg_id(u32::from_be_bytes([
                            first, second, third, fourth,
                        ])));
                    }
                }
            }
        }
        assert!(!is_korg_id(0));
        assert!(!is_korg_id(u32::from_be_bytes(*b"RUST")));
    }

    #[test]
    fn package_version_uses_korg_seven_bit_components() {
        assert_eq!(
            pack_version(1, 2, 3, Span::call_site()).unwrap(),
            0x0001_0203
        );
        assert_eq!(
            pack_version(127, 127, 127, Span::call_site()).unwrap(),
            0x007f_7f7f
        );
        assert!(pack_version(128, 0, 0, Span::call_site()).is_err());
        assert!(pack_version(0, 128, 0, Span::call_site()).is_err());
        assert!(pack_version(0, 0, 128, Span::call_site()).is_err());
    }
}
