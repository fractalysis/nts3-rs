# nts3-cli

Internal host tooling for the repository-root `./nts3.sh` Bash launcher. The
launcher starts this engine in the pinned container by default. This crate owns
package selection, target Cargo policy, GNU final linking, templates, command
transcripts, ELF/memory inspection, and the native initialization probe; it does
not orchestrate containers. Inspection uses the maintained `object` crate and
emits memory report schema v1.

Public commands and supported environments are documented in
[`../../docs/building.md`](../../docs/building.md).
