use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadFraction {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", decimal_places=1, fixed_fraction_bits=2)]
    value: Parameter,
}
fn main() {}
