# `nts3-macros`

Host-only procedural macros for the `nts3` facade. Plugin authors should depend
on and import `nts3`; it reexports `Nts3Parameters` through both its root and
prelude.

`syn`, `quote`, and `proc-macro2` run only while compiling macro input. Generated
parameter code contains fixed SDK arrays and static match dispatch, with no
parser dependency, heap allocation, trait object, or dynamic dispatch on the
NTS-3 target.

See [`../../docs/parameters.md`](../../docs/parameters.md) for the supported
parameter attribute syntax and index stability rules.
