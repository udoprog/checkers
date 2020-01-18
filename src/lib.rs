//! Checkers is a simple allocation checker for Rust that runs purely inside of
//! Rust.
//!
//! # Examples
//!
//! You use checkers by installing [`checkers::Allocator`] as your allocator,
//! then making use of either the [`#[checkers::test]`](crate::test) or
//! [`checkers::with!`] macros.
//!
//! [`checkers::Allocator`]: crate::Allocator
//! [`checkers::with!`]: crate::with
//!
//! ```rust,no_run
//! #[global_allocator]
//! static CHECKED: checkers::Allocator = checkers::Allocator;
//!
//! #[checkers::test]
//! fn test_allocations() {
//!     let _ = Box::into_raw(Box::new(42));
//! }
//!
//! #[test]
//! fn test_manually() {
//!     checkers::with!(|| {
//!         let _ = Box::into_raw(Box::new(42));
//!     });
//! }
//! ```

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    fmt, mem, ptr, slice,
};

mod machine;
pub use self::machine::Machine;
pub use checkers_macros::test;

thread_local! {
    /// Thread-local state required by the allocator.
    ///
    /// Feel free to interact with this directly, but it's primarily used
    /// through the [`test`](crate::test) macro.
    pub static STATE: ThreadLocalState = ThreadLocalState::new();
}

/// Verify the state of the allocator.
///
/// This currently performs the following tests:
/// * Checks that each allocation has an exact corresponding deallocation,
///   and that it happened _after_ the allocation it relates to.
/// * That there are no overlapping deallocations / allocations.
/// * That the thread-local timeline matches.
///
/// Will be enabled in the future:
/// * Check that the _global_ timeline matches (e.g. memory is sent to a
///   different thread, where it is de-allocated).
#[macro_export]
macro_rules! verify {
    ($state:expr) => {
        assert!(
            !$state.enabled.get(),
            "Cannot verify while allocator tracking is enabled"
        );

        let mut machine = $crate::Machine::default();

        let mut events = $state.events.borrow_mut();

        let mut any_errors = false;

        for event in events.as_slice() {
            if let Err(e) = machine.push(*event) {
                eprintln!("{}", e);
                any_errors = true;
            }
        }

        let regions = machine.trailing_regions();

        if !regions.is_empty() {
            eprintln!("Leaked regions:");

            for region in regions {
                eprintln!("{:?}", region);
            }

            any_errors = true;
        }

        events.clear();

        if any_errors {
            panic!("allocation checks failed");
        }
    };
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
pub struct Events {
    ptr: *mut Event,
    len: usize,
    cap: usize,
}

unsafe impl Sync for Events {}

impl Events {
    /// Construct a new collection of allocations.
    const fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }

    /// Grow the underlying collection.
    fn grow(&mut self) {
        let cap = self
            .cap
            .checked_mul(2)
            .expect("failed to calculate grown capacity");
        self.reserve(cap);
    }

    /// Access the layout of the collection
    unsafe fn layout(&self) -> Layout {
        Self::layout_with_cap(self.cap)
    }

    /// Calculate a layout based on the specified cap.
    unsafe fn layout_with_cap(cap: usize) -> Layout {
        let bytes_cap = cap
            .checked_mul(mem::size_of::<Event>())
            .expect("failed to calculate capacity");
        Layout::from_size_align_unchecked(bytes_cap, mem::align_of::<Event>())
    }

    /// Reserve and make sure there is enough space to store the specified
    /// number of events.
    pub fn reserve(&mut self, cap: usize) {
        if self.cap >= cap {
            return;
        }

        if self.ptr == ptr::null_mut() {
            unsafe {
                let layout = Self::layout_with_cap(cap);
                let new_ptr = System.alloc(layout);
                assert!(new_ptr != ptr::null_mut(), "allocation failed");
                self.ptr = new_ptr as *mut Event;
                self.cap = cap;
            }
        } else {
            unsafe {
                let layout = self.layout();
                let new_size = cap
                    .checked_mul(mem::size_of::<Event>())
                    .expect("failed to calculate capacity");

                let new_ptr = System.realloc(self.ptr as *mut u8, layout, new_size);
                assert!(!new_ptr.is_null(), "reallocation failed");
                self.ptr = new_ptr as *mut Event;
                self.cap = cap;
            }
        }
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice(&self) -> &[Event] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice_mut(&mut self) -> &mut [Event] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Clear the collection of events.
    pub fn clear(&mut self) {
        for e in self.as_slice_mut() {
            *e = Event::Empty;
        }

        self.len = 0;
    }

    /// Push a single allocation.
    fn push(&mut self, event: Event) {
        let n = self.len;

        while n >= self.cap {
            self.grow();
        }

        assert!(n < self.cap);
        assert!(self.ptr != ptr::null_mut());

        unsafe {
            *self.ptr.add(n) = event;
            self.len += 1;
        }
    }
}

/// Structure containing all thread-local state required to use the
/// single-threaded allocation checker.
pub struct ThreadLocalState {
    pub enabled: Cell<bool>,
    pub events: RefCell<Events>,
}

impl ThreadLocalState {
    /// Construct new local state.
    const fn new() -> Self {
        Self {
            enabled: Cell::new(false),
            events: RefCell::new(Events::new()),
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

    /// Reserve the specified number of events.
    pub fn reserve(&self, cap: usize) {
        self.events.borrow_mut().reserve(cap);
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

    /// Add the given offset to the current pointer.
    pub fn saturating_add(self, n: usize) -> Self {
        Self(self.0.saturating_add(n))
    }

    /// Test if pointer is aligned with the given argument.
    pub fn is_aligned_with(self, n: usize) -> bool {
        self.0 % n == 0
    }
}

impl fmt::Debug for Pointer {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", &(self.0 as *const ()))
    }
}

impl From<*mut u8> for Pointer {
    fn from(value: *mut u8) -> Self {
        Self(value as usize)
    }
}

impl From<usize> for Pointer {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Metadata for a single allocation or deallocation.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Placeholder for empty events.
    Empty,
    /// An allocation.
    Allocation {
        /// The pointer of the allocation.
        ptr: Pointer,
        /// The size of the allocation.
        size: usize,
        /// The alignment of the allocation.
        align: usize,
    },
    /// A deallocation.
    Deallocation {
        /// The pointer of the allocation.
        ptr: Pointer,
        /// The size of the allocation.
        size: usize,
        /// The alignment of the allocation.
        align: usize,
    },
}

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
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator;
/// ```
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);

        STATE.with(move |state| {
            if state.enabled.get() {
                state.events.borrow_mut().push(Event::Allocation {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                });
            }
        });

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATE.with(move |state| {
            if state.enabled.get() {
                state.events.borrow_mut().push(Event::Deallocation {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                });
            }
        });

        System.dealloc(ptr, layout);
    }
}
