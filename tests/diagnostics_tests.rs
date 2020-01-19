#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator;

#[test]
fn test_event_inspection() {
    let snapshot = checkers::with(|| {
        let _ = vec![1, 2, 3, 4];
    });

    assert_eq!(2, snapshot.events.len());
    assert!(snapshot.events[0].is_allocation_with(|r| r.size >= 16));
    assert!(snapshot.events[1].is_deallocation_with(|a| a.size >= 16));
    assert_eq!(1, snapshot.events.allocations());
    assert_eq!(1, snapshot.events.deallocations());
    assert!(snapshot.events.max_memory_used().unwrap() >= 16);
}
