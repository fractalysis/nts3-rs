use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadPolarity {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", curve_polarity="alternating")]
    value: Parameter,
}
fn main() {}
