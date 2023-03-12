use std::alloc::{GlobalAlloc, Layout, System};

/// Note: allocator which intentionally doesn't allocate a zeroed region.
struct TestAllocator;

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);

        if !checkers::is_muted() && layout.size() >= 32 {
            *ptr.add(31) = 1;
        }

        ptr
    }
}

#[global_allocator]
static ALLOCATOR: checkers::Allocator<TestAllocator> = checkers::Allocator::new(TestAllocator);

#[cfg(feature = "realloc")]
#[test]
fn test_realloc() {
    let snapshot = checkers::with(|| {
        let _ = vec![0u8; 32];
    });

    assert_eq!(1, snapshot.events.allocs());
    assert_eq!(1, snapshot.events.frees());

    // Note: is_zeroed is false since we intentionally corrupt it in the
    // allocator.
    assert!(snapshot.events[0]
        .is_alloc_zeroed_with(|r| r.is_zeroed == Some(false) && r.request.region.size == 32));
    assert!(snapshot.events[1].is_free_with(|r| r.size == 32));
}
