use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadMapping {
    #[parameter(name="BAD", min=0, max=10, default=5, parameter_type="none", mapping_min=-1)]
    value: Parameter,
}
fn main() {}
