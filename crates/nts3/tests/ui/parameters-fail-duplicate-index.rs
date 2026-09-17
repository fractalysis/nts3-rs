use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct DuplicateIndex {
    #[parameter(index=0, name="A", min=0, max=1, default=0, parameter_type="none")] a: Parameter,
    #[parameter(index=0, name="B", min=0, max=1, default=0, parameter_type="none")] b: Parameter,
}
fn main() {}
