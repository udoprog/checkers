use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;

/// Note: allocator which intentionally doesn't allocate a zeroed region.
struct TestAllocator;

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if checkers::is_muted() {
            return System.alloc(layout);
        }

        ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if checkers::is_muted() {
            return System.alloc_zeroed(layout);
        }

        ptr::null_mut()
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if checkers::is_muted() {
            return System.realloc(ptr, layout, new_size);
        }

        ptr::null_mut()
    }
}

#[global_allocator]
static ALLOCATOR: checkers::Allocator<TestAllocator> = checkers::Allocator::new(TestAllocator);

#[test]
fn test_failed_alloc_zeroed() {
    let layout = Layout::from_size_align(10, 1).unwrap();

    let snapshot = checkers::with(|| unsafe {
        assert_eq!(ptr::null_mut(), ALLOCATOR.alloc_zeroed(layout));
    });

    assert_eq!(1, snapshot.events.len());
    assert!(snapshot.events[0].is_failed());
}

#[test]
fn test_failed_alloc() {
    let layout = Layout::from_size_align(10, 1).unwrap();

    let snapshot = checkers::with(|| unsafe {
        assert_eq!(ptr::null_mut(), ALLOCATOR.alloc(layout));
    });

    assert_eq!(1, snapshot.events.len());
    assert!(snapshot.events[0].is_failed());
}

#[test]
fn test_failed_realloc() {
    let layout = Layout::from_size_align(10, 1).unwrap();

    let p = unsafe { ALLOCATOR.alloc(layout) };

    for n in 0..10 {
        unsafe {
            ptr::write(p.add(n), n as u8);
        }
    }

    assert!(!p.is_null());

    let snapshot = checkers::with(|| unsafe {
        // NB: These events will fail because checkers is enabled.
        assert_eq!(ptr::null_mut(), ALLOCATOR.realloc(p, layout, 20));
        ALLOCATOR.dealloc(p, layout);
    });

    assert_eq!(2, snapshot.events.len());
    assert!(snapshot.events[0].is_failed());
    assert!(
        snapshot.events[1].is_free_with(|r| r.size == layout.size() && r.align == layout.align())
    );
}
