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

mod machine;
pub use self::machine::Machine;

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

        let mut machine = $crate::Machine::default();

        for event in $state.events.borrow().as_slice() {
            if let Err(e) = machine.push(*event) {
                panic!("{}", e);
            }
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
    allocs: [Event; 1024],
    len: usize,
}

impl Events {
    /// Construct a new collection of allocations.
    const fn new() -> Self {
        Self {
            allocs: [Event::Empty; 1024],
            len: 0,
        }
    }

    /// Push a single allocation.
    fn allocation(&mut self, ptr: Pointer, layout: Layout) {
        let n = self.len;
        assert!(n < 1024);
        self.len += 1;

        self.allocs[n] = Event::Allocation {
            ptr,
            size: layout.size(),
            align: layout.align(),
        };
    }

    /// Push a single deallocation.
    fn deallocation(&mut self, ptr: Pointer, layout: Layout) {
        let n = self.len;
        assert!(n < 1024);
        self.len += 1;

        self.allocs[n] = Event::Deallocation {
            ptr,
            size: layout.size(),
            align: layout.align(),
        };
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice(&self) -> &[Event] {
        &self.allocs[..self.len]
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
                state.events.borrow_mut().allocation(ptr.into(), layout);
            }
        });

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATE.with(move |state| {
            if state.enabled.get() {
                state.events.borrow_mut().deallocation(ptr.into(), layout);
            }
        });

        System.dealloc(ptr, layout);
    }
}
