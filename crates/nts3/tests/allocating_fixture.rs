use nts3::{FrameworkAllocator, host::HostArena};

#[global_allocator]
static GLOBAL: FrameworkAllocator = FrameworkAllocator;

#[test]
fn vec_construction_is_served_only_while_arena_is_active() {
    let mut arena = HostArena::new(16 * 1024).unwrap();
    let stats = arena
        .run(|| {
            let mut values = Vec::<u64>::with_capacity(128);
            values.extend(0..128);
            assert_eq!(values[127], 127);
            // `values` drops before the arena reset. Individual deallocation is
            // intentionally a no-op, exactly as on target.
        })
        .unwrap();

    assert_eq!(stats.allocations, 1);
    assert_eq!(stats.requested_bytes, 1024);
    assert_eq!(stats.high_water, 1024);
}
