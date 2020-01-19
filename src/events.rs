//! Collections of events.
//!
//! We use a wrapper type to provide convenience methods for diagnostics.

use std::{ops, slice};

use crate::{Event, Machine, Violation};

/// A fixed-size collection of allocations.
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

    /// Push a single allocation.
    pub fn push(&mut self, event: Event) {
        // Note: pushing might allocate, so mute while we are doing that, if we
        // have to.

        if self.data.capacity() <= self.data.len() {
            let _g = crate::mute_guard(true);
            self.data.push(event);
        } else {
            self.data.push(event);
        }
    }

    /// Count the number of allocations in this collection of events.
    pub fn allocations(&self) -> usize {
        self.data
            .iter()
            .map(|e| match e {
                Event::Allocation { .. } => 1,
                _ => 0,
            })
            .sum()
    }

    /// Count the number of deallocations in this collection of events.
    pub fn deallocations(&self) -> usize {
        self.data
            .iter()
            .map(|e| match e {
                Event::Deallocation { .. } => 1,
                _ => 0,
            })
            .sum()
    }

    /// Validate the current state and populate the errors collection with any violations
    /// found.
    pub fn validate(&self, errors: &mut Vec<Violation>) {
        let mut machine = Machine::default();

        for event in self.as_slice() {
            if let Err(e) = machine.push(*event) {
                errors.push(e);
            }
        }

        for region in machine.trailing_regions() {
            errors.push(Violation::DanglingRegion { region });
        }
    }

    /// Max amount of memory used according to this event history.
    ///
    /// Returns the first violation encountered if the history is not sound.
    pub fn max_memory_used(&self) -> Result<usize, Violation> {
        let mut machine = Machine::default();

        let mut max = 0usize;

        for event in self.as_slice() {
            machine.push(*event)?;
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
