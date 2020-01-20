use std::alloc::{GlobalAlloc, Layout, System};

/// Note: allocator which intentionally doesn't copy all necessary bytes.
struct TestAllocator;

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);

        if !checkers::is_muted() && new_size == 8 {
            let v = *new_ptr.add(3);
            *new_ptr.add(3) = !v;
        }

        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: checkers::Allocator<TestAllocator> = checkers::Allocator::new(TestAllocator);

#[cfg(feature = "realloc")]
#[test]
fn test_realloc() {
    let snapshot = checkers::with(|| {
        let mut v = Vec::<u32>::new();
        v.reserve_exact(1);
        v.push(1);
        v.push(2);
    });

    assert_eq!(1, snapshot.events.allocs());
    assert_eq!(1, snapshot.events.reallocs());
    assert_eq!(1, snapshot.events.frees());

    assert!(snapshot.events[0].is_alloc_with(|r| r.size == 4));
    assert!(snapshot.events[1].is_realloc_with(|r| {
        // Note: not correctly relocated since we corrupted the third byte.
        r.is_relocated == Some(false) && r.free.size == 4 && r.alloc.size == 8
    }));
    assert!(snapshot.events[2].is_free_with(|r| r.size == 8));
}
