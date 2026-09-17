# nts3-cli

Internal host tooling for the repository-root `./nts3.sh` Bash launcher. The
launcher starts this engine in the pinned container by default. This crate owns
package selection, target Cargo policy, GNU final linking, templates, command
transcripts, and diagnostics; it does not orchestrate containers.

Public commands and supported environments are documented in
[`../../docs/building.md`](../../docs/building.md).
