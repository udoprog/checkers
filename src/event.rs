//! A single allocator event.

use crate::{AllocZeroed, Realloc, ReallocNull, Region, Request};

/// Metadata for a single allocation or deallocation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// An allocation.
    Alloc(Request),
    /// A deallocation.
    Free(Request),
    /// A zerod allocation, with an optional boolean indicates if it is actually
    /// zeroed or not.
    AllocZeroed(AllocZeroed),
    /// A reallocation that moves and resized memory from one location to
    /// another.
    Realloc(Realloc),
    /// An allocation failed (produced null).
    AllocFailed,
    /// A zero allocation that failed (produced null).
    AllocZeroedFailed,
    /// Allocator was asked to reallocate unallocated memory.
    ReallocNull(ReallocNull),
    /// A reallocation failed (produced null), and the previous region is left
    /// unchanged.
    ReallocFailed,
}

impl Event {
    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Region};
    ///
    /// let request = Request::without_backtrace(Region::new(100.into(), 100, 4));
    /// let event = Alloc(request);
    ///
    /// assert!(event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(!event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_alloc_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Alloc(request) | Self::AllocZeroed(AllocZeroed { request, .. }) => {
                f(request.region)
            }
            _ => false,
        }
    }

    /// Test if this event is a deallocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Region, Request};
    ///
    /// let request = Request::without_backtrace(Region::new(100.into(), 100, 4));
    /// let event = Free(request);
    ///
    /// assert!(!event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_free_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Free(request) => f(request.region),
            _ => false,
        }
    }

    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Region, AllocZeroed};
    ///
    /// let event = AllocZeroed(AllocZeroed::new(
    ///     Some(true),
    ///     Request::without_backtrace(Region::new(100.into(), 100, 4))
    /// ));
    ///
    /// assert!(event.is_alloc_zeroed_with(|r| r.request.region.size == 100 && r.request.region.align == 4));
    /// assert!(!event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_alloc_zeroed_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(&AllocZeroed) -> bool,
    {
        match self {
            Self::AllocZeroed(alloc_zeroed) => f(alloc_zeroed),
            _ => false,
        }
    }

    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Realloc, Region, Request};
    ///
    /// let event = Realloc(Realloc::without_backtrace(
    ///     Some(true),
    ///     Region::new(10.into(), 10, 1),
    ///     Region::new(20.into(), 20, 1)
    /// ));
    ///
    /// assert!(event.is_realloc_with(|r| r.free.size == 10 && r.alloc.size == 20));
    /// ```
    pub fn is_realloc_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(&Realloc) -> bool,
    {
        match self {
            Self::Realloc(realloc) => f(realloc),
            _ => false,
        }
    }

    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::Event;
    ///
    /// assert!(Event::AllocFailed.is_failed());
    /// assert!(Event::AllocZeroedFailed.is_failed());
    /// assert!(Event::ReallocFailed.is_failed());
    /// ```
    pub fn is_failed(&self) -> bool {
        match self {
            Self::AllocFailed { .. }
            | Self::AllocZeroedFailed { .. }
            | Self::ReallocFailed { .. } => true,
            _ => false,
        }
    }
}
