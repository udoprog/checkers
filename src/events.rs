//! Collections of events.
//!
//! We use a wrapper type to provide convenience methods for diagnostics.

use std::{ops, slice};

use crate::{Event, Machine, Violation};

/// Collections of events.
///
/// We use a wrapper type to provide convenience methods for diagnostics.
#[derive(Debug, Clone)]
pub struct Events {
    data: Vec<Event>,
}

impl Events {
    /// Construct a new collection of allocations.
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Get the number of events in this collection.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Test if collection is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Access the capacity of the Events container.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserve extra capacity for the underlying storage.
    pub fn reserve(&mut self, cap: usize) {
        self.data.reserve(cap.saturating_sub(self.data.capacity()));
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice(&self) -> &[Event] {
        ops::Deref::deref(self)
    }

    /// Fetch all allocations as a slice.
    pub fn as_slice_mut(&mut self) -> &mut [Event] {
        ops::DerefMut::deref_mut(self)
    }

    /// Clear the collection of events.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Push a single event into the collection of events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Events, Region};
    /// let mut events = Events::new();
    ///
    /// let request = Request::without_backtrace(Region::new(10.into(), 10, 1));
    /// events.push(Alloc(request));
    ///
    /// assert!(matches!(&events[0], &Alloc(..)));
    /// ```
    pub fn push(&mut self, event: Event) {
        // Note: pushing into an at-capacity collection would allocate, so we
        // take care of it here, while muting the tracker.
        if self.data.capacity() == self.data.len() {
            let _g = crate::mute_guard(true);
            self.data.reserve(1);
        }

        self.data.push(event);
    }

    /// Count the number of allocations in this collection of events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Events, Event, Region};
    /// let mut events = Events::new();
    ///
    /// let request = Request::without_backtrace(Region::new(10.into(), 10, 1));
    /// events.push(Alloc(request));
    ///
    /// assert_eq!(1, events.allocs());
    /// assert_eq!(0, events.frees());
    /// ```
    pub fn allocs(&self) -> usize {
        self.data
            .iter()
            .map(|e| match e {
                Event::Alloc { .. } | Event::AllocZeroed { .. } => 1,
                _ => 0,
            })
            .sum()
    }

    /// Count the number of allocations in this collection of events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Events, Region, Realloc};
    /// let mut events = Events::new();
    ///
    /// events.push(Realloc(Realloc::without_backtrace(
    ///     Some(true),
    ///     Region::new(10.into(), 10, 1),
    ///     Region::new(20.into(), 10, 1)
    /// )));
    ///
    /// assert_eq!(1, events.reallocs());
    /// assert_eq!(0, events.allocs());
    /// assert_eq!(0, events.frees());
    /// ```
    pub fn reallocs(&self) -> usize {
        self.data
            .iter()
            .map(|e| match e {
                Event::Realloc { .. } => 1,
                _ => 0,
            })
            .sum()
    }

    /// Count the number of frees in this collection of events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Events, Region, Request};
    /// let mut events = Events::new();
    ///
    /// let request = Request::without_backtrace(Region::new(10.into(), 10, 1));
    /// events.push(Free(request));
    ///
    /// assert_eq!(0, events.allocs());
    /// assert_eq!(1, events.frees());
    /// ```
    pub fn frees(&self) -> usize {
        self.data
            .iter()
            .map(|e| match e {
                Event::Free { .. } => 1,
                _ => 0,
            })
            .sum()
    }

    /// Validate the current state and populate the errors collection with any
    /// violations found.
    ///
    /// See [Machine::push] for more details on the kind of validation errors
    /// that can be raised.
    pub fn validate(&self, errors: &mut Vec<Violation>) {
        let mut machine = Machine::default();

        for event in self.as_slice() {
            if let Err(e) = machine.push(event) {
                errors.push(e);
            }
        }

        for alloc in machine.trailing_regions() {
            errors.push(Violation::Leaked { alloc });
        }
    }

    /// Max amount of memory used according to this event history.
    ///
    /// Returns the first violation encountered if the history is not sound.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use checkers::{Event::*, Events, Region, Request};
    /// let mut events = Events::new();
    ///
    /// let request = Request::without_backtrace(Region::new(0x10.into(), 0x10, 1));
    /// events.push(Alloc(request));
    ///
    /// let request = Request::without_backtrace(Region::new(0x20.into(), 0x10, 1));
    /// events.push(Alloc(request));
    ///
    /// let request = Request::without_backtrace(Region::new(0x10.into(), 0x10, 1));
    /// events.push(Free(request));
    ///
    /// assert_eq!(0x20, events.max_memory_used().unwrap());
    /// ```
    pub fn max_memory_used(&self) -> Result<usize, Violation> {
        let mut machine = Machine::default();

        let mut max = 0usize;

        for event in self.as_slice() {
            machine.push(event)?;
            max = usize::max(machine.memory_used, max);
        }

        Ok(max)
    }
}

impl ops::Deref for Events {
    type Target = [Event];

    fn deref(&self) -> &[Event] {
        ops::Deref::deref(&self.data)
    }
}

impl ops::DerefMut for Events {
    fn deref_mut(&mut self) -> &mut [Event] {
        ops::DerefMut::deref_mut(&mut self.data)
    }
}

impl<I: slice::SliceIndex<[Event]>> ops::Index<I> for Events {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        std::ops::Index::index(&self.data, index)
    }
}
