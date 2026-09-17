use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {}

mod first {
    use super::*;

    #[derive(Default)]
    struct First;

    #[nts3::plugin(
        name = "Duplicate One",
        developer_id = 0x5255_5354,
        unit_id = 1,
        sdram_bytes = 1
    )]
    impl Nts3Plugin for First {
        type Parameters = Parameters;
        fn process(&mut self, _: &mut Self::Parameters, _: &mut StereoBuffer<'_>) {}
    }
}

mod second {
    use super::*;

    #[derive(Default)]
    struct Second;

    #[nts3::plugin(
        name = "Duplicate Two",
        developer_id = 0x5255_5354,
        unit_id = 2,
        sdram_bytes = 1
    )]
    impl Nts3Plugin for Second {
        type Parameters = Parameters;
        fn process(&mut self, _: &mut Self::Parameters, _: &mut StereoBuffer<'_>) {}
    }
}

fn main() {}
