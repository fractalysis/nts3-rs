use nts3::{FrameworkAllocator, host::HostArena};

#[global_allocator]
static GLOBAL: FrameworkAllocator = FrameworkAllocator;

fn main() {
    let mut arena = HostArena::new(16 * 1024).expect("create host arena");
    let stats = arena
        .run(|| {
            // This ordinary Vec allocation is routed through exactly the same
            // aligned bump state used by the target global allocator.
            let mut values = Vec::<u64>::with_capacity(128);
            values.extend(0..128);
            assert_eq!(values.len(), 128);
            assert_eq!(values[127], 127);
            std::hint::black_box(values);
        })
        .expect("run host allocation probe");

    assert_eq!(stats.allocations, 1);
    assert_eq!(stats.requested_bytes, 128 * 8);
    assert!(stats.high_water <= stats.budget);
}
