use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct PlainSmoothing {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", smoothing_ms=10.0)]
    value: Parameter,
}
fn main() {}
