use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadAssignment {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", assign="z")]
    value: Parameter,
}
fn main() {}
