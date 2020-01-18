//! Fake machine implementation to validate an allocation history.

use std::{collections::BTreeMap, fmt};

use crate::{Event, Pointer};

pub enum PushError {
    EmptyEvent,
    AllocationOverlaps {
        requested: Region,
        existing: Region,
    },
    AllocationMisaligned {
        requested: Region,
    },
    DeallocateIncomplete {
        requested: Region,
        existing: Region,
    },
    DeallocateMisaligned {
        requested: Region,
        existing: Region,
    },
    DeallocateMissing {
        requested: Region,
        regions: Vec<Region>,
    },
}

impl fmt::Display for PushError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEvent => write!(fmt, "tried to push empty event."),
            Self::AllocationOverlaps {
                requested,
                existing,
            } => write!(
                fmt,
                "tried to allocate already allocated memory. requested: {:?}, existing: {:?}.",
                requested, existing
            ),
            Self::AllocationMisaligned { requested } => write!(
                fmt,
                "allocated misaligned region. requested: {:?}.",
                requested
            ),
            Self::DeallocateIncomplete {
                requested,
                existing,
            } => write!(
                fmt,
                "tried to deallocate only part of region. requested: {:?}, existing: {:?}.",
                requested, existing
            ),
            Self::DeallocateMisaligned {
                requested,
                existing,
            } => write!(
                fmt,
                "tried to deallocate misaligned region. requested: {:?}, existing: {:?}.",
                requested, existing
            ),
            Self::DeallocateMissing { requested, regions } => write!(
                fmt,
                "tried to deallocate missing region. requested: {:?}, regions: {:?}.",
                requested, regions
            ),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    ptr: Pointer,
    size: usize,
    align: usize,
}

impl fmt::Debug for Region {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{:?}-{:?} (size: {}, align: {})",
            self.ptr,
            self.ptr.saturating_add(self.size),
            self.size,
            self.align,
        )
    }
}

impl Region {
    pub fn new(ptr: Pointer, size: usize, align: usize) -> Self {
        Self { ptr, size, align }
    }

    /// Test if this region overlaps with another region.
    pub fn overlaps(self, other: Self) -> bool {
        self.ptr <= other.ptr && other.ptr <= self.ptr.saturating_add(self.size)
    }

    /// Test if regions are the same (minus alignment).
    pub fn is_same_region_as(self, other: Self) -> bool {
        self.ptr == other.ptr && self.size == other.size
    }
}

/// Fake machine implementation to validate an allocation history.
#[derive(Default)]
pub struct Machine {
    /// Used memory regions.
    regions: BTreeMap<Pointer, Region>,
}

impl Machine {
    /// Push an event into the machine.
    ///
    /// # Examples
    ///
    /// Checks for a double-free:
    ///
    /// ```rust
    /// use checkers::{Event, Machine};
    ///
    /// let mut machine = Machine::default();
    ///
    /// assert!(machine.push(Event::Allocation {
    ///     ptr: 0.into(),
    ///     size: 2,
    ///     align: 1,
    /// }).is_ok());
    ///
    /// assert!(machine.push(Event::Deallocation {
    ///     ptr: 0.into(),
    ///     size: 2,
    ///     align: 1,
    /// }).is_ok());
    ///
    /// assert!(machine.push(Event::Deallocation {
    ///     ptr: 0.into(),
    ///     size: 2,
    ///     align: 1,
    /// }).is_err());
    /// ```
    ///
    /// Checks for a misaligned allocation:
    ///
    /// ```rust
    /// use checkers::{Event, Machine};
    ///
    /// let mut machine = Machine::default();
    ///
    /// assert!(machine.push(Event::Allocation {
    ///     ptr: 5.into(),
    ///     size: 2,
    ///     align: 4,
    /// }).is_err());
    /// ```
    ///
    /// Tries to deallocate part of other region:
    ///
    /// ```rust
    /// use checkers::{Event, Machine};
    ///
    /// let mut machine = Machine::default();
    ///
    /// assert!(machine.push(Event::Allocation {
    ///     ptr: 100.into(),
    ///     size: 100,
    ///     align: 1,
    /// }).is_ok());
    ///
    /// assert!(machine.push(Event::Deallocation {
    ///     ptr: 150.into(),
    ///     size: 50,
    ///     align: 1,
    /// }).is_err());
    ///
    /// assert!(machine.push(Event::Deallocation {
    ///     ptr: 100.into(),
    ///     size: 50,
    ///     align: 1,
    /// }).is_err());
    /// ```
    pub fn push(&mut self, event: Event) -> Result<(), PushError> {
        match event {
            Event::Empty => return Err(PushError::EmptyEvent),
            Event::Allocation { ptr, size, align } => {
                let requested = Region::new(ptr, size, align);

                if !ptr.is_aligned_with(align) {
                    return Err(PushError::AllocationMisaligned { requested });
                }

                if let Some(existing) = find_region_overlaps(&self.regions, requested) {
                    return Err(PushError::AllocationOverlaps {
                        requested,
                        existing,
                    });
                }

                self.regions.insert(ptr, requested);
            }
            Event::Deallocation { ptr, size, align } => {
                let requested = Region::new(ptr, size, align);

                if let Some(existing) = find_region_overlaps(&self.regions, requested) {
                    if !existing.is_same_region_as(requested) {
                        return Err(PushError::DeallocateIncomplete {
                            requested,
                            existing,
                        });
                    }

                    if existing.align != requested.align {
                        return Err(PushError::DeallocateMisaligned {
                            requested,
                            existing,
                        });
                    }
                } else {
                    return Err(PushError::DeallocateMissing {
                        requested,
                        regions: self.regions.values().copied().collect(),
                    });
                }

                self.regions.remove(&ptr);
            }
        }

        return Ok(());

        fn find_region_overlaps(
            regions: &BTreeMap<Pointer, Region>,
            needle: Region,
        ) -> Option<Region> {
            if let Some((_, &region)) = regions.range(..=needle.ptr).next_back() {
                if region.overlaps(needle) {
                    return Some(region);
                }
            }

            if let Some((_, &region)) = regions.range(needle.ptr..).next() {
                if region.overlaps(needle) {
                    return Some(region);
                }
            }

            None
        }
    }
}
