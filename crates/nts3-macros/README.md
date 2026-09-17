# `nts3-macros`

Host-only procedural macros for the `nts3` facade. Plugin authors should depend
on and import `nts3`; it reexports `Nts3Parameters` through both its root and
prelude and exposes the `nts3::plugin` attribute.

`syn`, `quote`, and `proc-macro2` run only while compiling macro input. Generated
parameter code contains fixed SDK arrays and static match dispatch, with no
parser dependency, heap allocation, trait object, or dynamic dispatch on the
NTS-3 target. `#[nts3::plugin]` validates unit metadata and Cargo package
version, then emits one genericfx header, a versioned resource record, and all
required callbacks around the concrete plugin type. Applying it twice in one
crate is a compile error.

See [`../../docs/parameters.md`](../../docs/parameters.md) for the supported
parameter attribute syntax and index stability rules.
