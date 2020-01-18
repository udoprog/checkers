//! Checkers is a simple allocation checker for Rust that runs purely inside of
//! Rust.
//!
//! # Examples
//!
//! You use checkers by installing it's allocator, then making use of
//! [`checkers::with!`].
//!
//! [`checkers::with!`]: with!
//!
//! ```rust
//! #[global_allocator]
//! static CHECKED: checkers::Allocator = checkers::Allocator;
//!
//! #[test]
//! fn test_allocations() {
//!     checkers::with!(|| {
//!         let mut bytes = vec![10, 20, 30];
//!         bytes.truncate(2);
//!     });
//! }
//! ```

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    fmt,
};

thread_local! {
    /// Thread-local state required by the allocator.
    pub static STATE: ThreadLocalState = ThreadLocalState::new();
}

/// Verify the state of the allocator.
///
/// This currently performs the following tests:
/// * Checks that each allocation has an exact corresponding deallocation,
///   and that it happened _after_ the allocation it relates to.
///
/// More checks to be enabled in the future:
/// * That there are no overlapping deallocations / allocations.
/// * That the _global_ timeline matches.
#[macro_export]
macro_rules! verify {
    ($state:expr) => {
        assert!(
            !$state.enabled.get(),
            "Cannot verify while allocator tracking is enabled"
        );

        let mut deallocs = $state.deallocs.borrow().as_slice().to_vec();

        for alloc in $state.allocs.borrow().as_slice() {
            let index = match deallocs
                .iter()
                .position(|d| d.ptr == alloc.ptr && d.layout == alloc.layout)
            {
                Some(dealloc) => dealloc,
                None => panic!(
                    "No matching deallocation found for allocation {:?} - deallocations: {:?}",
                    alloc, deallocs
                ),
            };

            deallocs.remove(index);
        }

        if !deallocs.is_empty() {
            panic!(
                "Found {} deallocations without allocations: {:?}",
                deallocs.len(),
                deallocs
            );
        }
    }
}

/// Run the given function inside of the allocation checker.
///
/// Thread-local checking will be enabled for the duration of the closure, then
/// disabled and verified at _the end_ of the closure.
///
/// # Examples
///
/// ```rust
/// #[test]
/// fn test_dealloc_layout() {
///     checkers::with(|| {
///        let mut bytes = Bytes::from(vec![10, 20, 30]);
///        bytes.truncate(2);
///     });
/// }
/// ```
#[macro_export]
macro_rules! with {
    ($f:expr) => {
        $crate::STATE.with(move |state| {
            state.with($f);
            $crate::verify!(state);
        });
    };
}

/// A fixed-size collection of allocations.
pub struct Allocations {
    allocs: [AllocationMeta; 1024],
    len: usize,
}

impl Allocations {
    /// Construct a new collection of allocations.
    const fn new() -> Self {
        Self {
            allocs: [AllocationMeta::new(); 1024],
            len: 0,
        }
    }

    /// Push a single allocation.
    fn push(&mut self, ptr: Pointer, layout: Layout, step: usize) {
        let n = self.len;
        assert!(n < 1024);
        self.len += 1;

        self.allocs[n].ptr = ptr;
        self.allocs[n].layout = Some(layout);
        self.allocs[n].step = step;
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice(&self) -> &[AllocationMeta] {
        &self.allocs[..self.len]
    }
}

/// Structure containing all thread-local state required to use the
/// single-threaded allocation checker.
pub struct ThreadLocalState {
    pub enabled: Cell<bool>,
    timeline: Cell<usize>,
    pub allocs: RefCell<Allocations>,
    pub deallocs: RefCell<Allocations>,
}

impl ThreadLocalState {
    /// Construct new local state.
    const fn new() -> Self {
        Self {
            enabled: Cell::new(false),
            timeline: Cell::new(0),
            allocs: RefCell::new(Allocations::new()),
            deallocs: RefCell::new(Allocations::new()),
        }
    }

    /// Wrap the given closure in an enabled allocation tracking state.
    pub fn with<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        self.enabled.set(true);
        let _guard = Guard(self);
        f();

        struct Guard<'a>(&'a ThreadLocalState);

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.enabled.set(false);
            }
        }
    }

    /// Step the timeline, returning the next value.
    pub fn step(&self) -> usize {
        let n = self.timeline.get() + 1;
        self.timeline.set(n);
        n
    }
}

/// A type-erased pointer.
/// The inner representation is specifically _not_ a raw pointer to avoid
/// aliasing the pointers handled by the allocator.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pointer(usize);

impl Pointer {
    /// Construct a new default poitner.
    pub const fn new() -> Self {
        Self(0)
    }
}

impl fmt::Debug for Pointer {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", &(self.0 as *const ()))
    }
}

impl From<*mut u8> for Pointer {
    fn from(value: *mut u8) -> Pointer {
        Pointer(value as usize)
    }
}

/// Metadata for a single allocation or deallocation.
#[derive(Clone, Copy)]
pub struct AllocationMeta {
    /// The pointer of the allocation.
    pub ptr: Pointer,
    /// The layout of the allocation.
    pub layout: Option<Layout>,
    /// The linear step at which this allocation happened.
    /// This is a global counter which increases for every allocation and
    /// deallocation, which can be used to verify that they happen in the
    /// expected order.
    pub step: usize,
}

impl fmt::Debug for AllocationMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(layout) = &self.layout {
            write!(
                fmt,
                "{:?} {{size: {}, align: {}}}",
                &self.ptr,
                layout.size(),
                layout.align()
            )
        } else {
            write!(fmt, "{:?} {{?}}", &self.ptr)
        }
    }
}

impl AllocationMeta {
    /// Construct a new default allocation metadata.
    const fn new() -> Self {
        Self {
            ptr: Pointer::new(),
            layout: None,
            step: 0,
        }
    }
}

/// Allocator that needs to be installed.
///
/// You install it by doing:
///
/// ```rust,no_run
/// #[global_allocator]
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator;
/// ```
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);

        STATE.with(move |state| {
            if state.enabled.get() {
                let step = state.step();
                state.allocs.borrow_mut().push(ptr.into(), layout, step);
            }
        });

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATE.with(move |state| {
            if state.enabled.get() {
                let step = state.step();
                state.deallocs.borrow_mut().push(ptr.into(), layout, step);
            }
        });

        System.dealloc(ptr, layout);
    }
}
