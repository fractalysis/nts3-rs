# Third-party notices

## wrl/baseplug smoother

`crates/nts3/src/parameter.rs` adapts the `Smooth<f32>` one-pole smoothing
algorithm from wrl/baseplug:

- repository: <https://github.com/wrl/baseplug>
- pinned commit: `9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74`
- source: <https://github.com/wrl/baseplug/blob/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74/src/smooth.rs>
- upstream license: MIT OR Apache-2.0
- license selected for this reuse: MIT

The adaptation specializes the implementation to `f32` and `no_std`, uses
`libm::expf`, replaces Baseplug's fixed-size output array with per-sample output,
retains only the first output of each render block for compatible status timing,
and defines nonpositive smoothing times/sample rates as immediate. A host-only
reference implementation in the same source file compares outputs and status
transitions against the pinned fixed-block algorithm.

Baseplug's gradient translation and declick implementations were reviewed but
not copied. NTS-3 hardware metadata already supplies mapping curves, while the
framework only needs linear raw-to-normalized conversion. Declick stages
arbitrary value swaps and would add state/API not required by integer parameter
smoothing.

### MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
