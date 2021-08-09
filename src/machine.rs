//! Fake machine implementation to validate an allocation history.

use std::{
    collections::{btree_map as map, BTreeMap},
    fmt,
};

use crate::{Alloc, AllocZeroed, Event, Pointer, Realloc, Violation};

/// A memory region. Including its location in memory `ptr`, it's `size` and
/// alignment `align`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Region {
    /// The pointer of the allocation.
    pub ptr: Pointer,
    /// The size of the allocation.
    pub size: usize,
    /// The alignment of the allocation.
    pub align: usize,
}

impl Region {
    /// Construct a new region.
    pub fn new(ptr: Pointer, size: usize, align: usize) -> Self {
        Self { ptr, size, align }
    }

    /// Test if this region overlaps with another region.
    pub fn overlaps(self, other: Self) -> bool {
        self.ptr <= other.ptr && other.ptr < self.ptr.saturating_add(self.size)
    }

    /// Test if regions are the same (minus alignment).
    pub fn is_same_region_as(self, other: Self) -> bool {
        self.ptr == other.ptr && self.size == other.size
    }
}

impl fmt::Display for Region {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}-{} (size: {}, align: {})",
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
    regions: BTreeMap<Pointer, Alloc>,
    /// Current memory used according to allocations.
    pub memory_used: usize,
}

impl Machine {
    /// Push an event into the machine.
    ///
    /// # Examples
    ///
    /// Checks for a double-free:
    ///
    /// ```rust
    /// use checkers::{Event::*, Alloc, Region, Machine};
    ///
    /// let mut machine = Machine::default();
    ///
    /// assert!(machine.push(&Alloc(Alloc::without_backtrace(Region::new(0.into(), 2, 1)))).is_ok());
    /// assert!(machine.push(&Free(Region::new(0.into(), 2, 1))).is_ok());
    /// assert!(machine.push(&Free(Region::new(0.into(), 2, 1))).is_err());
    /// ```
    ///
    /// Check for a misaligned allocation:
    ///
    /// ```rust
    /// use checkers::{Event::*, Alloc, Region, Machine, Violation};
    ///
    /// let mut machine = Machine::default();
    /// let requested = Region::new(5.into(), 2, 4);
    ///
    /// assert!(matches!(
    ///     machine.push(&Alloc(Alloc::without_backtrace(requested))).unwrap_err(),
    ///     Violation::MisalignedAlloc { .. }
    /// ));
    /// ```
    ///
    /// Tries to deallocate part of other region:
    ///
    /// ```rust
    ///
    /// use checkers::{Event::*, Alloc, Region, Machine, Violation};
    /// let mut machine = Machine::default();
    /// let existing = Region::new(100.into(), 100, 1);
    ///
    /// assert!(machine.push(&Alloc(Alloc::without_backtrace(existing))).is_ok());
    ///
    /// let requested = Region::new(150.into(), 50, 1);
    /// assert!(matches!(
    ///     machine.push(&Free(requested)).unwrap_err(),
    ///     Violation::MissingFree { .. }
    /// ));
    ///
    /// let requested = Region::new(100.into(), 50, 1);
    /// assert!(matches!(
    ///     machine.push(&Free(requested)).unwrap_err(),
    ///     Violation::IncompleteFree { .. }
    /// ));
    /// ```
    pub fn push(&mut self, event: &Event) -> Result<(), Violation> {
        match event {
            Event::Alloc(requested) => {
                self.alloc(requested)?;
            }
            Event::Free(requested) => {
                self.free(*requested)?;
            }
            Event::AllocZeroed(AllocZeroed {
                is_zeroed,
                alloc: requested,
            }) => {
                if let Some(false) = is_zeroed {
                    return Err(Violation::NonZeroedAlloc {
                        requested: requested.region,
                    });
                }

                self.alloc(requested)?;
            }
            Event::Realloc(Realloc {
                is_relocated,
                free,
                alloc,
            }) => {
                if let Some(false) = is_relocated {
                    return Err(Violation::NonCopiedRealloc {
                        free: *free,
                        alloc: alloc.region,
                    });
                }

                self.free(*free)?;
                self.alloc(alloc)?;
            }
            Event::ReallocNull => {
                return Err(Violation::ReallocNull {});
            }
            // Note: the following have no effects, outside of what the erorrs
            // mean to the caller of the allocator. They could for example
            // decide to gracefully signal OOM (https://github.com/rust-lang/rust/issues/48043)
            // or panic.
            Event::AllocFailed => (),
            Event::AllocZeroedFailed => (),
            Event::ReallocFailed => (),
        }

        Ok(())
    }

    /// Process an allocation.
    fn alloc(&mut self, requested: &Alloc) -> Result<(), Violation> {
        if !requested.region.ptr.is_aligned_with(requested.region.align) {
            return Err(Violation::MisalignedAlloc {
                requested: requested.region,
            });
        }

        if let Some(existing) = find_region_overlaps(&self.regions, requested.region).next() {
            return Err(Violation::ConflictingAlloc {
                requested: requested.region,
                existing: existing.region,
            });
        }

        self.memory_used = self.memory_used.saturating_add(requested.region.size);

        let existing = self.regions.insert(
            requested.region.ptr,
            Alloc {
                region: requested.region,
                backtrace: requested.backtrace.clone(),
            },
        );

        debug_assert!(existing.is_none());
        Ok(())
    }

    /// Process a free.
    fn free(&mut self, requested: Region) -> Result<(), Violation> {
        let entry = if let map::Entry::Occupied(entry) = self.regions.entry(requested.ptr) {
            entry
        } else {
            return Err(Violation::MissingFree { requested });
        };

        let existing = entry.get();

        if !existing.region.is_same_region_as(requested) {
            return Err(Violation::IncompleteFree {
                requested,
                existing: existing.region,
            });
        }

        if existing.region.align != requested.align {
            return Err(Violation::MisalignedFree {
                requested,
                existing: existing.region,
            });
        }

        let (_, region) = entry.remove_entry();
        self.memory_used = self.memory_used.saturating_sub(region.region.size);
        Ok(())
    }

    /// Access all trailing regions (ones which have not been deallocated).
    pub fn trailing_regions(&self) -> Vec<Alloc> {
        self.regions.values().cloned().collect()
    }
}

/// Utility function to find overlapping regions.
fn find_region_overlaps(
    regions: &BTreeMap<Pointer, Alloc>,
    needle: Region,
) -> impl Iterator<Item = Alloc> + '_ {
    let head = regions
        .range(..=needle.ptr)
        .take_while(move |(_, r)| r.region.overlaps(needle));

    let tail = regions
        .range(needle.ptr..)
        .take_while(move |(_, r)| r.region.overlaps(needle));

    head.chain(tail).map(|(_, r)| r.clone())
}
