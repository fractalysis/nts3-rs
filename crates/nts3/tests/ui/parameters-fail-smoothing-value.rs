use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadSmoothing {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", smoothing_ms=0.0)]
    value: SmoothedParameter,
}
fn main() {}
