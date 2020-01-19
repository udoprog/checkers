//! Allocator that needs to be installed.
//!
//! Delegates allocations to [`std::alloc::System`] (this might be configurable
//! in the future).
//!
//! [`std::alloc::System`]: std::alloc::System
//!
//! You install it by doing:
//!
//! ```rust,no_run
//! #[global_allocator]
//! static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
//! ```

use crate::{Event, Region};
use std::alloc::{GlobalAlloc, Layout, System};

pub struct Allocator<T = System> {
    delegate: T,
}

impl<T> Allocator<T> {
    /// Construct an allocator with a custom delegate global allocator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[global_allocator]
    /// static ALLOCATOR: checkers::Allocator = checkers::Allocator::new(std::alloc::System);
    /// ```
    pub const fn new(delegate: T) -> Allocator<T> {
        Allocator { delegate }
    }
}

impl Allocator<System> {
    /// Construct an allocator with the system delegate global allocator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[global_allocator]
    /// static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
    /// ```
    pub const fn system() -> Allocator<System> {
        Allocator { delegate: System }
    }
}

unsafe impl<T> GlobalAlloc for Allocator<T>
where
    T: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.delegate.alloc(layout);

        if !crate::is_muted() {
            crate::with_state(move |s| {
                s.borrow_mut().events.push(Event::Alloc(Region {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                }));
            });
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !crate::is_muted() {
            crate::with_state(move |s| {
                s.borrow_mut().events.push(Event::Free(Region {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                }));
            });
        }

        self.delegate.dealloc(ptr, layout);
    }
}
