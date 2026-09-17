trait NotAPlugin {}
struct Plugin;

#[nts3::plugin(name = "Wrong Trait", developer_id = 1, unit_id = 1, sdram_bytes = 1)]
impl NotAPlugin for Plugin {}

fn main() {}
