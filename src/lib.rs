//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/checkers-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/checkers)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/checkers.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/checkers)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-checkers-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/checkers)
//! [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/checkers/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/udoprog/checkers/actions?query=branch%3Amain)
//!
//! Checkers is a simple allocation sanitizer for Rust. It plugs in through the
//! [global allocator] and can sanity check your unsafe Rust during integration
//! testing. Since it plugs in through the global allocator it doesn't require any
//! additional dependencies and works for all platforms - but it is more limited in
//! what it can verify.
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
//! <br>
//!
//! ## Usage
//!
//! Add `checkers` as a dev-dependency to your project:
//!
//! ```toml
//! checkers = "0.6.2"
//! ```
//!
//! Replace the global allocator in a [test file] and wrap tests you wish to
//! memory sanitise with `#[checkers::test]`:
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
//! > Note that it's important that you write your test as an *integration test*
//! > by adding it to your `tests/` folder to isolate the use of the global
//! > allocator.
//!
//! <br>
//!
//! ## Safety
//!
//! With the default feature set, this library performs diagnostics which will
//! produce undefined behavior. Therefore, it is recommended that you only use
//! checkers for _testing_, and never in any production code.
//!
//! If you want to avoid this, you'll have to disable the `realloc` and `zeroed`
//! features, but this will also produce less actionable diagnostics.
//!
//! In a future release, this behavior will be changed to be opt-in through feature
//! flags instead of enabled by default.
//!
//! <br>
//!
//! ## Features
//!
//! The following are features available, that changes how checkers work.
//!
//! * `realloc` - Enabling this feature causes checker to verify that a [realloc]
//!   operation is correctly implemented. That bytes from the old region were
//!   faithfully transferred to the new, resized one.
//!   Since this can have a rather significant performance impact, it can be
//!   disabled.
//!   Note that this will produce undefined behavior ([#1]) by reading uninitialized
//!   memory, and should only be enabled to provide diagnostics on a best-effort
//!   basis.
//! * `zeroed` - Enabling this feature causes checkers to verify that a call to
//!   [alloc_zeroed] produces a region where all bytes are _set_ to zero.
//!   Note that if the underlying allocator is badly implemented this will produce
//!   undefined behavior ([#1]) since it could read uninitialized memory.
//! * `macros` - Enables dependencies and re-exports of macros, like
//!   [`#[checkers::test]`][checkers-test].
//! * `backtrace` - Enables the capture and rendering of backtraces. If
//!   disabled, any fields containing backtraces will be `None`.
//!
//! [realloc]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#method.realloc
//! [alloc_zeroed]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#method.alloc_zeroed
//! [#1]: https://github.com/udoprog/checkers/issues/1
//!
//! <br>
//!
//! ## Examples
//!
//! It is recommended that you use checkers for [integration tests], which by
//! default lives in the `./tests` directory. Each file in this directory will be
//! compiled as a separate program, so the use of the global allocator can be more
//! isolated.
//!
//! We then use checkers by installing
//! [`checkers::Allocator`][checkers-allocator] as the global allocator, after
//! this we can make use of [`#[checkers::test]`][checkers-test] attribute macro
//! or the [`checkers::with`][checkers-with] function in our tests.
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
//! With [`checkers::with`][checkers-with], we can perform more detailed
//! diagnostics:
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
//!     assert_eq!(1, snapshot.events.allocs());
//!     assert_eq!(1, snapshot.events.frees());
//!     assert!(snapshot.events.max_memory_used().unwrap() >= 16);
//! }
//! ```
//!
//! [test file]: https://doc.rust-lang.org/cargo/guide/project-layout.html
//! [checkers-allocator]: https://docs.rs/checkers/latest/checkers/struct.Allocator.html
//! [checkers-test]: https://docs.rs/checkers/latest/checkers/attr.test.html
//! [checkers-with]: https://docs.rs/checkers/latest/checkers/fn.with.html
//! [global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html
//! [integration tests]: https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests
//! [mismatched layout]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety
//! [see test]: https://github.com/udoprog/checkers/blob/master/tests/leaky_tests.rs

#![deny(missing_docs)]

use std::cell::{Cell, RefCell};
use std::fmt;

mod allocator;
#[cfg(feature = "backtrace")]
#[path = "bt/impl.rs"]
mod bt;
#[cfg(not(feature = "backtrace"))]
#[path = "bt/mock.rs"]
mod bt;
mod event;
mod events;
mod machine;
mod utils;
mod violation;

pub use self::allocator::Allocator;
pub use self::event::Event;
pub use self::events::Events;
pub use self::machine::{Machine, Region};
pub use self::violation::Violation;
#[cfg(feature = "macros")]
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

/// Test if the crate is currently muted. The allocator is muted by default.
///
/// We mute the allocator for allocations we don't want to be tracked. This is
/// useful to avoid tracing internal allocations.
///
/// # Examples
///
/// ```rust
/// assert!(checkers::is_muted());
///
/// {
///     let _g = checkers::mute_guard(false);
///     assert!(!checkers::is_muted());
/// }
///
/// assert!(checkers::is_muted());
///
/// checkers::with_unmuted(|| {
///     assert!(!checkers::is_muted());
/// });
///
/// assert!(checkers::is_muted());
///
/// let result = std::panic::catch_unwind(|| {
///     let _g = checkers::mute_guard(false);
///     assert!(!checkers::is_muted());
///     panic!("uh oh");
/// });
/// assert!(result.is_err());
/// assert!(checkers::is_muted());
/// ```
pub fn is_muted() -> bool {
    MUTED.with(Cell::get)
}

/// Enable muting for the duration of the guard. A guard ensures that the muted
/// state is restored to its original value even if we are unwinding due to a
/// panic. You should prefer to use [with_unmuted] when possible.
///
/// See [is_muted] for details on what this means.
pub fn mute_guard(muted: bool) -> MuteGuard {
    MuteGuard(MUTED.with(|s| s.replace(muted)))
}

/// Run the given closure while the allocator is unmuted.
///
/// See [is_muted] for details on what this means.
pub fn with_unmuted<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _g = crate::mute_guard(false);
    f()
}

/// Run the given closure while the allocator is muted. This can be used to
/// whitelist sections of code where the allocation checker should be disabled.
///
/// See [is_muted] for details on what this means.
///
/// # Examples
///
/// ```rust
/// #[global_allocator]
/// static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();
///
/// lazy_static::lazy_static! {
///    pub static ref EX: Box<u32> = checkers::with_muted(|| Box::new(123));
/// }
///
/// let snapshot = checkers::with(|| {
///     let _ = &*EX;
/// });
///
/// // Snapshot can be successfully verified since we're excluding the static
/// // allocation from analysis.
/// checkers::verify!(snapshot);
/// ```
pub fn with_muted<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _g = crate::mute_guard(true);
    f()
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
///    assert_eq!(1, state.events.allocs());
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
        $crate::with_muted(|| {
            let mut validations = Vec::new();
            $state.validate(&mut validations);

            for e in &validations {
                eprintln!("{}", e);
            }

            if !validations.is_empty() {
                panic!("allocation checks failed");
            }
        });
    };
}

/// A snapshot of the state of the checkers allocator.
#[derive(Debug)]
pub struct Snapshot {
    /// Snapshot of all collected events.
    pub events: Events,
}

impl Snapshot {
    /// Validate the current snapshot.
    ///
    /// See [Events::validate] for more documentation.
    pub fn validate(&self, errors: &mut Vec<Violation>) {
        self.events.validate(errors);
    }
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
/// assert_eq!(1, snapshot.events.allocs());
/// assert_eq!(1, snapshot.events.frees());
/// assert!(snapshot.events.max_memory_used().unwrap() >= 16);
/// ```
pub fn with<F>(f: F) -> Snapshot
where
    F: FnOnce(),
{
    crate::with_state(|s| {
        s.borrow_mut().events.clear();

        crate::with_unmuted(f);

        let snapshot = Snapshot {
            events: s.borrow().events.clone(),
        };

        snapshot
    })
}

/// Structure containing all thread-local state required to use the
/// single-threaded allocation checker.
pub struct State {
    /// Events collected.
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

/// Metadata about an allocation request.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Request {
    /// The allocated region.
    pub region: Region,
    /// Captured backtrace if present.
    pub backtrace: Option<crate::bt::Backtrace>,
}

impl Request {
    /// Construct a new allocation without a complete backtrace.
    pub fn without_backtrace(region: Region) -> Self {
        Self {
            region,
            backtrace: None,
        }
    }
}

/// Description of an allocation that is zeroed by the allocator.
///
/// Zeroed allocation are guaranteed by the allocator to be zeroed.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AllocZeroed {
    /// Indicates if the region was indeed zeroed.
    pub is_zeroed: Option<bool>,
    /// The region that was allocated.
    pub request: Request,
}

impl AllocZeroed {
    /// Construct a new reallocation.
    pub fn new(is_zeroed: Option<bool>, request: Request) -> Self {
        Self { is_zeroed, request }
    }
}

/// Description of a reallocation.
///
/// Reallocations frees one location in memory and copies the shared prefix.
/// If the region is the same size or smaller, it can usually be performed
/// in-place.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Realloc {
    /// Indicates if the subset of the old region was faithfully copied over
    /// to the new region.
    pub is_relocated: Option<bool>,
    /// The region that was freed.
    pub free: Region,
    /// The region that was allocated.
    pub alloc: Region,
    /// Backtrace of the reallocation request.
    pub backtrace: Option<crate::bt::Backtrace>,
}

impl Realloc {
    /// Construct a new reallocation without a backtrace.
    pub fn without_backtrace(is_relocated: Option<bool>, free: Region, alloc: Region) -> Self {
        Self {
            is_relocated,
            free,
            alloc,
            backtrace: None,
        }
    }

    /// Construct a new reallocation.
    pub fn new(
        is_relocated: Option<bool>,
        free: Region,
        alloc: Region,
        backtrace: Option<crate::bt::Backtrace>,
    ) -> Self {
        Self {
            is_relocated,
            free,
            alloc,
            backtrace,
        }
    }

    pub(crate) fn free(&self) -> Request {
        Request {
            region: self.free,
            backtrace: self.backtrace.clone(),
        }
    }

    pub(crate) fn alloc(&self) -> Request {
        Request {
            region: self.alloc,
            backtrace: self.backtrace.clone(),
        }
    }
}

/// Description of a null reallocation. These are always considered errors.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReallocNull {
    /// Backtrace of the reallocation request.
    pub backtrace: Option<crate::bt::Backtrace>,
}
