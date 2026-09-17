use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct LongName {
    #[parameter(name="ABCDEFGHIJKLMNOPQRSTUV", min=0, max=1, default=0, parameter_type="none")]
    value: Parameter,
}
fn main() {}
