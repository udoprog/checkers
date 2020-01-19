#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[test]
fn test_event_inspection() {
    let snapshot = checkers::with(|| {
        let _ = vec![1, 2, 3, 4];
    });

    assert_eq!(2, snapshot.events.len());
    assert!(snapshot.events[0].is_alloc_with(|r| r.size >= 16));
    assert!(snapshot.events[1].is_free_with(|a| a.size >= 16));
    assert_eq!(1, snapshot.events.allocs());
    assert_eq!(1, snapshot.events.frees());
    assert!(snapshot.events.max_memory_used().unwrap() >= 16);
}
