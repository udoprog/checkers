//! Checkers is a simple allocation checker for Rust. It plugs in through the
//! [global allocator] API and can sanity check your unsafe Rust during
//! integration testing.
//!
//! [global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html
//!
//! It can check for the following things:
//! * Double-frees.
//! * Attempts to free regions which are not allocated.
//! * Underlying allocator producting regions not adhering to the requested layout.
//!   Namely size and alignment.
//! * Other arbitrary user-defined conditions ([see test]).
//!
//! What it can't do:
//! * Test multithreaded code. Since the allocator is global, it is difficult to
//!  scope the state for each test case.
//!
//! [see test]: https://github.com/udoprog/checkers/blob/master/tests/leaky_tests.rs
//!
//! # Examples
//!
//! You use checkers by installing [`checkers::Allocator`] as your allocator,
//! then making use of either the [`#[checkers::test]`](attr.test.html) or
//! [`checkers::with!`] macros.
//!
//! [`checkers::Allocator`]: crate::Allocator
//! [`checkers::with!`]: macro.with.html
//!
//! ```rust,no_run
//! #[global_allocator]
//! static ALLOCATOR: checkers::Allocator = checkers::Allocator;
//!
//! #[checkers::test]
//! fn test_leak_box() {
//!     let _ = Box::into_raw(Box::new(42));
//! }
//! ```
//!
//! The above would result in the following test output:
//!
//! ```text
//! dangling region: 0x226e5784f30-0x226e5784f40 (size: 16, align: 8).
//! thread 'test_leak_box' panicked at 'allocation checks failed', tests\leaky_tests.rs:4:1
//! ```

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    fmt, thread,
};

mod machine;
pub use self::machine::{Machine, Region, Violation};
pub use checkers_macros::test;

thread_local! {
    /// Thread-local state required by the allocator.
    ///
    /// Feel free to interact with this directly, but it's primarily used
    /// through the [`test`](crate::test) macro.
    pub static STATE: RefCell<State> = RefCell::new(State::new());
    pub static MUTED: Cell<bool> = Cell::new(true);
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
        let mut validations = Vec::new();
        $state.validate(&mut validations);

        for e in &validations {
            eprintln!("{:?}", e);
        }

        if !validations.is_empty() {
            panic!("allocation checks failed");
        }
    };
}

/// Simplified helper macro to run the checkers environment over a closure.
///
/// # Examples
///
/// ```rust
/// checkers::with!(|| {
///     let _ = Box::into_raw(Box::new(0u128));
/// });
/// ```
#[macro_export]
macro_rules! with {
    ($f:expr) => {
        checkers::STATE.with(|state| state.borrow_mut().clear());

        (|| {
            let _g = checkers::mute(false);
            ($f)();
        })();

        checkers::STATE.with(|state| {
            $crate::verify!(&mut *state.borrow_mut());
        });
    };
}

/// A fixed-size collection of allocations.
pub struct Events {
    data: Vec<Event>,
}

impl Events {
    /// Construct a new collection of allocations.
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Reserve extra capacity for the underlying storage.
    pub fn reserve(&mut self, cap: usize) {
        self.data.reserve(cap.saturating_sub(self.data.capacity()));
    }

    /// Access the capacity of the Events container.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice(&self) -> &[Event] {
        &self.data[..]
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice_mut(&mut self) -> &mut [Event] {
        &mut self.data[..]
    }

    /// Clear the collection of events.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Push a single allocation.
    pub fn push(&mut self, event: Event) {
        // Note: pushing might allocate, so mute while we are doing that, if we
        // have to.

        if self.data.capacity() <= self.data.len() {
            let _g = crate::mute(true);
            self.data.push(event);
        } else {
            self.data.push(event);
        }
    }
}

/// Structure containing all thread-local state required to use the
/// single-threaded allocation checker.
pub struct State {
    events: Events,
}

impl State {
    /// Construct new local state.
    pub const fn new() -> Self {
        Self {
            events: Events::new(),
        }
    }

    /// Reserve the specified number of events.
    pub fn reserve(&mut self, cap: usize) {
        self.events.reserve(cap);
    }

    /// Validate the current state and populate the errors collection with any violations
    /// found.
    pub fn validate(&self, errors: &mut Vec<Violation>) {
        let mut machine = Machine::default();

        for event in self.events.as_slice() {
            if let Err(e) = machine.push(*event) {
                errors.push(e);
            }
        }

        for region in machine.trailing_regions() {
            errors.push(Violation::DanglingRegion { region });
        }
    }

    /// Clear the current collection of events.
    pub fn clear(&mut self) {
        self.events.clear();
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

        if !thread::panicking() && !crate::is_muted() {
            STATE.with(move |state| {
                state.borrow_mut().events.push(Event::Allocation {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                });
            });
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !thread::panicking() && !crate::is_muted() {
            STATE.with(move |state| {
                state.borrow_mut().events.push(Event::Deallocation {
                    ptr: ptr.into(),
                    size: layout.size(),
                    align: layout.align(),
                });
            });
        }

        System.dealloc(ptr, layout);
    }
}

/// Test if the crate is currently muted.
pub fn is_muted() -> bool {
    MUTED.with(|s| s.get())
}

/// Enable muting for the duration of the guard.
pub fn mute(muted: bool) -> StateGuard {
    StateGuard(MUTED.with(|s| s.replace(muted)))
}

/// A helper guard to make sure the state is de-allocated on drop.
pub struct StateGuard(bool);

impl Drop for StateGuard {
    fn drop(&mut self) {
        MUTED.with(|s| s.set(self.0));
    }
}
