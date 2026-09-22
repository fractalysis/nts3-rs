# Parameters

Import `nts3::prelude::*`, derive `Nts3Parameters`, and annotate every named
field with one `#[parameter(...)]` attribute. Parameter structs support at most
eight fields. The derive generates `Default`, static eight-slot SDK descriptor
and mapping tables, bounds-checked index dispatch, and smoother lifecycle hooks.
Unused slots use the SDK's canonical none values.

```rust
#[derive(Nts3Parameters)]
struct Parameters {
    #[parameter(
        name = "TIME",
        min = 1,
        max = 2000,
        default = 500,
        parameter_type = "milliseconds",
        smoothing_ms = 100.0,
        assign = "x",
        curve = "exp",
        mapping_min = 1,
        mapping_max = 2000,
        mapping_default = 500
    )]
    time: SmoothedParameter,
}
```

Fields must be `Parameter` or `SmoothedParameter`. The latter requires a finite,
positive `smoothing_ms`; that value is the one-pole time constant, not a linear
ramp duration.

## Attribute options

Required options are `name`, `min`, `max`, `default`, and `parameter_type`.
`min`, `max`, `default`, optional `center`, and mapping values must fit `i16`.
The default and center must be inside `min..=max`. A missing center defaults to
`min`.

Supported display names are `none`, `percent`, `db`, `cents`, `semi` or
`semitones`, `oct` or `octaves`, `hertz`, `khertz` or `kilohertz`, `bpm`,
`msec` or `milliseconds`, `sec` or `seconds`, `enum`, `drywet`, `pan`,
`spread`, `onoff`, and `midi_note`. SDK custom-string display parameters are
deferred until the framework has an explicit static string-table API; using
`parameter_type = "strings"` is rejected rather than returning a null string.

Use either `decimal_places = 0..15` or `fixed_fraction_bits = 0..15`, not both.
Names are nonempty, at most 21 bytes, and use the SDK character set: ASCII
letters, digits, spaces, hyphens, and underscores.

Mapping options default to the descriptor range/default, assignment `none`,
curve `linear`, and polarity `unipolar`. Assignments are `none`, `x`, `y`, and
`depth`. Curves are `linear`, `exp`, `log`, `toggle`, `minclip`, and `maxclip`.
`curve_polarity` is `unipolar` or `bipolar`. A `drywet` parameter assigned to
`depth` must explicitly use `curve = "exp"` and
`curve_polarity = "bipolar"`; the derive rejects other combinations because
they do not produce a working NTS-3 FX DEPTH dry/wet mapping. Other parameter
types may use the depth control with any supported curve. Mapping endpoints
may be inverted, but both endpoints and `mapping_default` must remain inside
the descriptor range.

## Stable indices and presets

By default a field's index is its declaration order. Reordering, inserting, or
removing implicitly indexed fields changes the host-visible parameter IDs and
can break saved presets.

For a reorder-safe source layout, set `index = N` explicitly. Indices must be
unique and contiguous from zero; gaps and duplicates are compile errors. Once a
unit is released, preserve the semantic meaning of every index even if fields
are reordered in Rust.

The generated target implementation uses static matches and fixed arrays. It
performs no heap allocation or dynamic dispatch. `syn`, `quote`, `trybuild`, and
other parser/test dependencies execute only on the build host and are not linked
into target artifacts.
