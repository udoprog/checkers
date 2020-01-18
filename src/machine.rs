//! Fake machine implementation to validate an allocation history.

use std::{
    collections::{btree_map as map, BTreeMap},
    fmt,
};

use crate::{Event, Pointer};

pub enum Violation {
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
        overlaps: Vec<Region>,
    },
    DanglingRegion {
        region: Region,
    },
}

impl Violation {
    /// Test that this violation refers to a dangling region and that it matches
    /// the given predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use checkers::{Region, Violation};
    /// let violation = Violation::DanglingRegion {
    ///     region: Region::new(42.into(), 20, 4),
    /// };
    /// assert!(violation.is_dangling_region(|r| r.size == 20 && r.align == 4));
    ///
    /// let violation = Violation::EmptyEvent;
    /// assert!(!violation.is_dangling_region(|r| true));
    /// ```
    pub fn is_dangling_region<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match *self {
            Self::DanglingRegion { region } => f(region),
            _ => false,
        }
    }
}

impl fmt::Debug for Violation {
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
            Self::DeallocateMissing {
                requested,
                overlaps,
            } => write!(
                fmt,
                "tried to deallocate missing region. requested: {:?}, overlaps: {:?}.",
                requested, overlaps
            ),
            Self::DanglingRegion { region } => write!(fmt, "dangling region: {:?}.", region,),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    pub ptr: Pointer,
    pub size: usize,
    pub align: usize,
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
    pub fn push(&mut self, event: Event) -> Result<(), Violation> {
        match event {
            Event::Empty => return Err(Violation::EmptyEvent),
            Event::Allocation { ptr, size, align } => {
                let requested = Region::new(ptr, size, align);

                if !ptr.is_aligned_with(align) {
                    return Err(Violation::AllocationMisaligned { requested });
                }

                if let Some(existing) = find_region_overlaps(&self.regions, requested).next() {
                    return Err(Violation::AllocationOverlaps {
                        requested,
                        existing,
                    });
                }

                debug_assert!(self.regions.insert(ptr, requested).is_none());
            }
            Event::Deallocation { ptr, size, align } => {
                let requested = Region::new(ptr, size, align);

                if let map::Entry::Occupied(entry) = self.regions.entry(ptr) {
                    let existing = *entry.get();

                    if !existing.is_same_region_as(requested) {
                        return Err(Violation::DeallocateIncomplete {
                            requested,
                            existing,
                        });
                    }

                    if existing.align != requested.align {
                        return Err(Violation::DeallocateMisaligned {
                            requested,
                            existing,
                        });
                    }

                    entry.remove_entry();
                    return Ok(());
                }

                let overlaps = find_region_overlaps(&self.regions, requested).collect();

                return Err(Violation::DeallocateMissing {
                    requested,
                    overlaps,
                });
            }
        }

        return Ok(());

        fn find_region_overlaps<'a>(
            regions: &'a BTreeMap<Pointer, Region>,
            needle: Region,
        ) -> impl Iterator<Item = Region> + 'a {
            let head = regions
                .range(..=needle.ptr)
                .take_while(move |(_, &r)| r.overlaps(needle));

            let tail = regions
                .range(needle.ptr..)
                .take_while(move |(_, &r)| r.overlaps(needle));

            head.chain(tail).map(|(_, &r)| r)
        }
    }

    /// Access all trailing regions (ones which have not been deallocated).
    pub fn trailing_regions(&self) -> Vec<Region> {
        self.regions.values().copied().collect()
    }
}
