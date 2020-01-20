use crate::{AllocZeroed, Event, Realloc, Region};
use std::alloc::{GlobalAlloc, Layout, System};

/// Allocator that needs to be installed.
///
/// Delegates allocations to [`std::alloc::System`] (this might be configurable
/// in the future).
///
/// [`std::alloc::System`]: std::alloc::System
///
/// You install it by doing:
///
/// ```rust,no_run
/// #[global_allocator]
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
/// ```
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
        Self::new(System)
    }
}

unsafe impl<T> GlobalAlloc for Allocator<T>
where
    T: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.delegate.alloc(layout);

        // Note: return early, caller is likely to panic or handle OOM scenario.
        // gracefully.
        // TODO: Consider emitting diagnostics.
        if ptr.is_null() {
            return ptr;
        }

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

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.delegate.alloc_zeroed(layout);

        // Note: return early, caller is likely to panic or handle OOM scenario.
        // gracefully.
        // TODO: Consider emitting diagnostics.
        if ptr.is_null() {
            return ptr;
        }

        if !crate::is_muted() {
            crate::with_state(move |s| {
                #[cfg(feature = "zeroed")]
                let is_zeroed = Some(crate::utils::is_zeroed_ptr(ptr, layout.size()));
                #[cfg(not(feature = "zeroed"))]
                let is_zeroed = None;

                s.borrow_mut().events.push(Event::AllocZeroed(AllocZeroed {
                    is_zeroed,
                    alloc: Region {
                        ptr: ptr.into(),
                        size: layout.size(),
                        align: layout.align(),
                    },
                }));
            });
        }

        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if crate::is_muted() {
            return self.delegate.realloc(ptr, layout, new_size);
        }

        // Safety Note: This needs to happen before call to `realloc`, since it
        // might deallocate it.
        #[cfg(feature = "realloc")]
        let min_size = usize::min(layout.size(), new_size);
        #[cfg(feature = "realloc")]
        let old_hash = {
            assert!(!ptr.is_null());
            crate::utils::hash_ptr(ptr, min_size)
        };

        // Safety Note: Convert to pointer early to avoid relying on potentially
        // dangling pointer later.
        let old_ptr = ptr.into();
        let new_ptr = self.delegate.realloc(ptr, layout, new_size);

        // Note: return early, caller is likely to panic or handle OOM scenario.
        // gracefully. Prior memory is unaltered.
        // TODO: Consider emitting diagnostics.
        if new_ptr.is_null() {
            return new_ptr;
        }

        crate::with_state(move |s| {
            #[cfg(feature = "realloc")]
            let is_relocated = Some(old_hash == crate::utils::hash_ptr(new_ptr, min_size));
            #[cfg(not(feature = "realloc"))]
            let is_relocated = None;

            let free = Region {
                ptr: old_ptr,
                size: layout.size(),
                align: layout.align(),
            };

            let alloc = Region {
                ptr: new_ptr.into(),
                size: new_size,
                align: layout.align(),
            };

            s.borrow_mut().events.push(Event::Realloc(Realloc {
                is_relocated,
                free,
                alloc,
            }));
        });

        new_ptr
    }
}
