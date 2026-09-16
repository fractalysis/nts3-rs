# Pinned implementation references

## Baseplug smoother

- Repository: <https://github.com/wrl/baseplug>
- Commit: `9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74`
- Original path: `src/smooth.rs`
- Permanent source: <https://github.com/wrl/baseplug/blob/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74/src/smooth.rs>
- Upstream package license: `MIT OR Apache-2.0`
- License selected for reuse here: MIT

`baseplug-smooth.rs` is an unmodified local reference copy. It is not intended
to compile as part of this project. Agent 05 will adapt its core f32 one-pole
algorithm to `no_std`, variable NTS-3 render sizes, and per-sample `next_*`
access while preserving behavior through golden tests.

Any adapted production source must retain the MIT notice, pinned provenance and
a clear modification statement. `baseplug-LICENSE-MIT` is copied from the same
commit.
