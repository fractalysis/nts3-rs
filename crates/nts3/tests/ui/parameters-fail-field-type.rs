use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadFieldType {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none")]
    value: i16,
}
fn main() {}
