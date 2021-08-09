//! A single allocator event.

use crate::{Alloc, AllocZeroed, Realloc, Region};

/// Metadata for a single allocation or deallocation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// An allocation.
    Alloc(Alloc),
    /// A deallocation.
    Free(Region),
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
    ReallocNull,
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
    /// use checkers::{Alloc, Event, Region};
    /// let event = Event::Alloc(Alloc::without_backtrace(Region::new(100.into(), 100, 4)));
    ///
    /// assert!(event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(!event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_alloc_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Alloc(Alloc { region, .. })
            | Self::AllocZeroed(AllocZeroed {
                alloc: Alloc { region, .. },
                ..
            }) => f(*region),
            _ => false,
        }
    }

    /// Test if this event is a deallocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let event = checkers::Event::Free(checkers::Region::new(100.into(), 100, 4));
    ///
    /// assert!(!event.is_alloc_with(|r| r.size == 100 && r.align == 4));
    /// assert!(event.is_free_with(|r| r.size == 100 && r.align == 4));
    /// ```
    pub fn is_free_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Free(region) => f(*region),
            _ => false,
        }
    }

    /// Test if this event is an allocation which matches the specified
    /// predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Alloc, Event, Region, AllocZeroed};
    /// let event = Event::AllocZeroed(AllocZeroed::new(
    ///     Some(true),
    ///     Alloc::without_backtrace(Region::new(100.into(), 100, 4))
    /// ));
    ///
    /// assert!(event.is_alloc_zeroed_with(|r| r.alloc.region.size == 100 && r.alloc.region.align == 4));
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
    /// use checkers::{Event::*, Alloc, Region, Realloc};
    ///
    /// let event = Realloc(Realloc::new(
    ///     Some(true),
    ///     Region::new(10.into(), 10, 1),
    ///     Alloc::without_backtrace(Region::new(20.into(), 20, 1))
    /// ));
    ///
    /// assert!(event.is_realloc_with(|r| r.free.size == 10 && r.alloc.region.size == 20));
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
