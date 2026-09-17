use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadCurve {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", curve="spline")]
    value: Parameter,
}
fn main() {}
