//! Checkers is a simple allocation sanitizer for Rust. It plugs in through the
//! [global allocator] and can sanity check your unsafe Rust during integration
//! testing. Since it plugs in through the global allocator it doesn't require any
//! additional dependencies and works for all platforms - but it is more limited in
//! what it can verify.
//!
//! [global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html
//!
//! It can check for the following things:
//! * Double-frees.
//! * Memory leaks.
//! * Freeing regions which are not allocated.
//! * Freeing only part of regions which are allocated.
//! * Freeing a region with a [mismatched layout].
//! * That the underlying allocator produces regions adhering to the requested
//!   layout. Namely size and alignment.
//! * Detailed information on memory usage.
//! * Other user-defined conditions ([see test]).
//!
//! What it can't do:
//! * Test multithreaded code. Since the allocator is global, it is difficult to
//!   scope the state for each test case.
//! * Detect out-of-bounds accesses.
//!
//! [mismatched layout]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety
//! [see test]: https://github.com/udoprog/checkers/blob/master/tests/leaky_tests.rs
//!
//! # Examples
//!
//! It is recommended that you use checkers for [integration tests], which by
//! default lives in the `./tests` directory. Each file in this directory will be
//! compiled as a separate program, so the use of the global allocator can be more
//! isolated.
//!
//! [integration tests]: https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests
//!
//! We then use checkers by installing [`checkers::Allocator`] as the global
//! allocator, after this we can make use of [`#[checkers::test]`](attr.test.html) attribute macro or
//! the [`checkers::with`] function in our tests.
//!
//! [`checkers::Allocator`]: crate::Allocator
//! [`checkers::with`]: crate::with
//!
//! ```rust
//! #[global_allocator]
//! static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
//!
//! #[checkers::test]
//! fn test_allocations() {
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
//!
//! With `checkers::with`, we can perform more detailed diagnostics:
//!
//! ```rust
//! #[global_allocator]
//! static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
//!
//! #[test]
//! fn test_event_inspection() {
//!     let snapshot = checkers::with(|| {
//!         let _ = vec![1, 2, 3, 4];
//!     });
//!
//!     assert_eq!(2, snapshot.events.len());
//!     assert!(snapshot.events[0].is_alloc_with(|r| r.size >= 16));
//!     assert!(snapshot.events[1].is_free_with(|a| a.size >= 16));
//!     assert_eq!(1, snapshot.events.allocations());
//!     assert_eq!(1, snapshot.events.frees());
//!     assert!(snapshot.events.max_memory_used().unwrap() >= 16);
//! }
//! ```

use std::{
    cell::{Cell, RefCell},
    fmt,
};

mod allocator;
mod events;
mod machine;

pub use self::allocator::Allocator;
pub use self::events::Events;
pub use self::machine::{Machine, Region, Violation};
pub use checkers_macros::test;

thread_local! {
    /// Thread-local state required by the allocator.
    ///
    /// Feel free to interact with this directly, but it's primarily used
    /// through the [`test`](crate::test) macro.
    static STATE: RefCell<State> = RefCell::new(State::new());
    static MUTED: Cell<bool> = Cell::new(true);
}

/// Perform an operation, while having access to the thread-local state.
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&RefCell<State>) -> R,
{
    crate::STATE.with(f)
}

/// Test if the crate is currently muted.
pub fn is_muted() -> bool {
    MUTED.with(Cell::get)
}

/// Enable muting for the duration of the guard.
pub fn mute_guard(muted: bool) -> MuteGuard {
    MuteGuard(MUTED.with(|s| s.replace(muted)))
}

/// A helper guard to make sure the state is de-allocated on drop.
pub struct MuteGuard(bool);

impl Drop for MuteGuard {
    fn drop(&mut self) {
        MUTED.with(|s| s.set(self.0));
    }
}

/// Verify the state of the allocator.
///
/// Note: this macro is used by default if the `verify` parameter is not
/// specified in [`#[checkers::test]`](attr.test.html).
///
/// Currently performs the following tests:
/// * Checks that each allocation has an exact corresponding deallocation,
///   and that it happened _after_ the allocation it relates to.
/// * That there are no overlapping frees / allocations.
/// * That the thread-local timeline matches.
///
/// # Examples
///
/// ```rust
/// #[global_allocator]
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
///
/// fn verify_test_custom_verify(state: &mut checkers::State) {
///    assert_eq!(1, state.events.allocations());
///    checkers::verify!(state);
/// }
///
/// #[checkers::test(verify = "verify_test_custom_verify")]
/// fn test_custom_verify() {
///     let _ = Box::into_raw(vec![1, 2, 3, 4, 5].into_boxed_slice());
/// }
/// ```
#[macro_export]
macro_rules! verify {
    ($state:expr) => {
        let mut validations = Vec::new();
        $state.validate(&mut validations);

        for e in &validations {
            eprintln!("{}", e);
        }

        if !validations.is_empty() {
            panic!("allocation checks failed");
        }
    };
}

pub struct Snapshot {
    pub events: Events,
}

/// Run the specified closure and return a snapshot of the memory state
/// afterwards.
///
/// This can be useful to programmatically test for allocation invariants,
/// while the default behavior is to simply panic on invariant errors.
///
/// # Examples
///
/// ```rust
/// #[global_allocator]
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
///
/// let snapshot = checkers::with(|| {
///     let _ = vec![1, 2, 3, 4];
/// });
///
/// assert_eq!(2, snapshot.events.len());
/// assert!(snapshot.events[0].is_alloc_with(|a| a.size >= 16));
/// assert!(snapshot.events[1].is_free_with(|a| a.size >= 16));
/// assert_eq!(1, snapshot.events.allocations());
/// assert_eq!(1, snapshot.events.frees());
/// assert!(snapshot.events.max_memory_used().unwrap() >= 16);
/// ```
pub fn with<F>(f: F) -> Snapshot
where
    F: FnOnce(),
{
    crate::with_state(|s| {
        s.borrow_mut().events.clear();

        {
            let _g = crate::mute_guard(false);
            f();
        }

        let snapshot = Snapshot {
            events: s.borrow().events.clone(),
        };

        snapshot
    })
}

/// Structure containing all thread-local state required to use the
/// single-threaded allocation checker.
pub struct State {
    pub events: Events,
}

impl State {
    /// Construct new local state.
    pub const fn new() -> Self {
        Self {
            events: Events::new(),
        }
    }

    /// Reserve the specified number of events.
    ///
    /// See [Events::reserve] for more documentation.
    pub fn reserve(&mut self, cap: usize) {
        self.events.reserve(cap);
    }

    /// Clear the current collection of events.
    ///
    /// See [Events::clear] for more documentation.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Validate the current state.
    ///
    /// See [Events::validate] for more documentation.
    pub fn validate(&self, errors: &mut Vec<Violation>) {
        self.events.validate(errors);
    }
}

/// A type-erased pointer.
/// The inner representation is specifically _not_ a raw pointer to avoid
/// aliasing the pointers handled by the allocator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl fmt::Display for Pointer {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Event {
    /// An allocation.
    Alloc(Region),
    /// A deallocation.
    Free(Region),
}

impl Event {
    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let event = checkers::Event::Alloc(checkers::Region {
    ///     ptr: 100.into(),
    ///     size: 100,
    ///     align: 4,
    /// });
    ///
    /// assert!(event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(!event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_alloc_with<F>(self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Alloc(region) => f(region),
            _ => false,
        }
    }

    /// Test if this event is a deallocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let event = checkers::Event::Free(checkers::Region {
    ///     ptr: 100.into(),
    ///     size: 100,
    ///     align: 4,
    /// });
    ///
    /// assert!(!event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_free_with<F>(self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Free(region) => f(region),
            _ => false,
        }
    }
}
