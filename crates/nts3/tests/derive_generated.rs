use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct EchoParameters {
    #[parameter(
        name = "TIME",
        min = 1,
        max = 2000,
        default = 500,
        parameter_type = "milliseconds",
        smoothing_ms = 100.0,
        assign = "x",
        curve = "exp",
        mapping_min = 1,
        mapping_max = 2000,
        mapping_default = 500
    )]
    time_ms: SmoothedParameter,

    #[parameter(
        name = "FEEDBACK",
        min = 0,
        max = 1000,
        default = 0,
        parameter_type = "percent",
        decimal_places = 1,
        assign = "y",
        curve = "linear",
        mapping_min = 0,
        mapping_max = 1000,
        mapping_default = 0
    )]
    feedback: Parameter,
}

#[derive(Nts3Parameters)]
struct ExplicitlyIndexed {
    #[parameter(
        index = 1,
        name = "SECOND",
        min = -10,
        max = 10,
        center = 0,
        default = 2,
        parameter_type = "pan",
        fixed_fraction_bits = 2,
        assign = "depth",
        curve = "log",
        curve_polarity = "bipolar",
        mapping_min = 10,
        mapping_max = -10,
        mapping_default = 0
    )]
    second: Parameter,

    #[parameter(
        index = 0,
        name = "FIRST",
        min = 0,
        max = 1,
        default = 1,
        parameter_type = "onoff",
        curve = "toggle"
    )]
    first: Parameter,
}

fn bytes_of<T>(value: &T) -> &[u8] {
    // SAFETY: an immutable byte view covering exactly one initialized `T` is
    // valid for the duration of the shared borrow. The generated SDK structs
    // contain only integer/byte fields and every byte is initialized.
    unsafe {
        core::slice::from_raw_parts((value as *const T).cast::<u8>(), core::mem::size_of::<T>())
    }
}

#[test]
fn smooth_echo_metadata_has_exact_sdk_bytes_and_padding() {
    assert_eq!(EchoParameters::COUNT, 2);

    let descriptor_bytes = bytes_of(&EchoParameters::DESCRIPTORS);
    let mut expected_descriptors = [0u8; 8 * 32];
    expected_descriptors[0..2].copy_from_slice(&1i16.to_le_bytes());
    expected_descriptors[2..4].copy_from_slice(&2000i16.to_le_bytes());
    expected_descriptors[4..6].copy_from_slice(&1i16.to_le_bytes());
    expected_descriptors[6..8].copy_from_slice(&500i16.to_le_bytes());
    expected_descriptors[8] = 9;
    expected_descriptors[9] = 0;
    expected_descriptors[10..14].copy_from_slice(b"TIME");

    let feedback = 32;
    expected_descriptors[feedback + 2..feedback + 4].copy_from_slice(&1000i16.to_le_bytes());
    expected_descriptors[feedback + 8] = 1;
    expected_descriptors[feedback + 9] = 0x11;
    expected_descriptors[feedback + 10..feedback + 18].copy_from_slice(b"FEEDBACK");
    assert_eq!(descriptor_bytes, expected_descriptors);

    let mapping_bytes = bytes_of(&EchoParameters::MAPPINGS);
    let mut expected_mappings = [0u8; 8 * 8];
    expected_mappings[0] = 1;
    expected_mappings[1] = 1;
    expected_mappings[2..4].copy_from_slice(&1i16.to_le_bytes());
    expected_mappings[4..6].copy_from_slice(&2000i16.to_le_bytes());
    expected_mappings[6..8].copy_from_slice(&500i16.to_le_bytes());
    expected_mappings[8] = 2;
    expected_mappings[9] = 0;
    expected_mappings[12..14].copy_from_slice(&1000i16.to_le_bytes());
    assert_eq!(mapping_bytes, expected_mappings);
}

#[test]
fn defaults_dispatch_and_smoothing_come_from_attributes() {
    let mut parameters = EchoParameters::default();
    assert_eq!(parameters.time_ms.raw(), 500);
    assert_eq!(parameters.feedback.raw(), 0);
    assert_eq!(parameters.get(0), Some(500));
    assert_eq!(parameters.get(1), Some(0));
    assert_eq!(parameters.get(2), None);

    parameters.initialize_smoothers(48_000.0);
    assert!(parameters.set(0, 2_000));
    assert!(parameters.set(1, 2_000));
    assert!(!parameters.set(8, 20));
    assert_eq!(parameters.get(0), Some(2_000));
    assert_eq!(parameters.get(1), Some(1_000));

    parameters.begin_block();
    let first = parameters.time_ms.next_plain();
    parameters.end_block();
    assert!(first > 500.0 && first < 2_000.0);
    parameters.reset_smoothers();
    assert_eq!(parameters.time_ms.current_plain(), 2_000.0);
}

#[test]
fn explicit_indices_define_metadata_and_dispatch_order() {
    assert_eq!(ExplicitlyIndexed::COUNT, 2);
    assert_eq!(ExplicitlyIndexed::DESCRIPTORS[0].name()[..5], *b"FIRST");
    assert_eq!(ExplicitlyIndexed::DESCRIPTORS[1].name()[..6], *b"SECOND");
    assert_eq!(ExplicitlyIndexed::DESCRIPTORS[1].center(), 0);
    assert_eq!(ExplicitlyIndexed::DESCRIPTORS[1].format().raw(), 2);
    assert_eq!(ExplicitlyIndexed::MAPPINGS[1].curve().raw(), 0x82);
    assert_eq!(ExplicitlyIndexed::MAPPINGS[1].min(), 10);
    assert_eq!(ExplicitlyIndexed::MAPPINGS[1].max(), -10);

    let mut parameters = ExplicitlyIndexed::default();
    assert_eq!(parameters.get(0), Some(1));
    assert_eq!(parameters.get(1), Some(2));
    parameters.set(0, 0);
    parameters.set(1, -7);
    assert_eq!(parameters.first.raw(), 0);
    assert_eq!(parameters.second.raw(), -7);
}
