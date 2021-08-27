//! Fake machine implementation to validate an allocation history.

use std::{
    collections::{btree_map as map, BTreeMap},
    fmt,
};

use crate::{AllocZeroed, Event, Pointer, Request, Violation};

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
    regions: BTreeMap<Pointer, Request>,
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
    /// use checkers::{Event::*, Request, Region, Machine};
    ///
    /// let mut machine = Machine::default();
    ///
    /// let request = Request::without_backtrace(Region::new(0.into(), 2, 1));
    /// assert!(machine.push(&Alloc(request)).is_ok());
    ///
    /// let request = Request::without_backtrace(Region::new(0.into(), 2, 1));
    /// assert!(machine.push(&Free(request)).is_ok());
    ///
    /// let request = Request::without_backtrace(Region::new(0.into(), 2, 1));
    /// assert!(machine.push(&Free(request)).is_err());
    /// ```
    ///
    /// Check for a misaligned allocation:
    ///
    /// ```rust
    /// use checkers::{Event::*, Request, Region, Machine, Violation};
    ///
    /// let mut machine = Machine::default();
    /// let region = Region::new(5.into(), 2, 4);
    ///
    /// let request = Request::without_backtrace(region);
    /// assert!(matches!(
    ///     machine.push(&Alloc(request)).unwrap_err(),
    ///     Violation::MisalignedAlloc { .. }
    /// ));
    /// ```
    ///
    /// Tries to deallocate part of other region:
    ///
    /// ```rust
    ///
    /// use checkers::{Event::*, Request, Region, Machine, Violation};
    /// let mut machine = Machine::default();
    /// let existing = Region::new(100.into(), 100, 1);
    ///
    /// let request = Request::without_backtrace(existing);
    /// assert!(machine.push(&Alloc(request)).is_ok());
    ///
    /// let request = Request::without_backtrace(Region::new(150.into(), 50, 1));
    /// assert!(matches!(
    ///     machine.push(&Free(request)).unwrap_err(),
    ///     Violation::MissingFree { .. }
    /// ));
    ///
    /// let request = Request::without_backtrace(Region::new(100.into(), 50, 1));
    /// assert!(matches!(
    ///     machine.push(&Free(request)).unwrap_err(),
    ///     Violation::IncompleteFree { .. }
    /// ));
    /// ```
    pub fn push(&mut self, event: &Event) -> Result<(), Violation> {
        match event {
            Event::Alloc(requested) => {
                self.alloc(requested)?;
            }
            Event::Free(requested) => {
                self.free(requested)?;
            }
            Event::AllocZeroed(AllocZeroed { is_zeroed, request }) => {
                if let Some(false) = is_zeroed {
                    return Err(Violation::NonZeroedAlloc {
                        alloc: request.clone(),
                    });
                }

                self.alloc(request)?;
            }
            Event::Realloc(realloc) => {
                if let Some(false) = realloc.is_relocated {
                    return Err(Violation::NonCopiedRealloc {
                        realloc: realloc.clone(),
                    });
                }

                self.free(&realloc.free())?;
                self.alloc(&realloc.alloc())?;
            }
            Event::ReallocNull(realloc) => {
                return Err(Violation::ReallocNull {
                    realloc: realloc.clone(),
                });
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
    fn alloc(&mut self, request: &Request) -> Result<(), Violation> {
        if !request.region.ptr.is_aligned_with(request.region.align) {
            return Err(Violation::MisalignedAlloc {
                alloc: request.clone(),
            });
        }

        if let Some(existing) = find_region_overlaps(&self.regions, request.region).next() {
            return Err(Violation::ConflictingAlloc {
                request: request.clone(),
                existing,
            });
        }

        self.memory_used = self.memory_used.saturating_add(request.region.size);

        let existing = self.regions.insert(request.region.ptr, request.clone());

        debug_assert!(existing.is_none());
        Ok(())
    }

    /// Process a free.
    fn free(&mut self, request: &Request) -> Result<(), Violation> {
        let entry = if let map::Entry::Occupied(entry) = self.regions.entry(request.region.ptr) {
            entry
        } else {
            return Err(Violation::MissingFree {
                request: request.clone(),
            });
        };

        let existing = entry.get();

        if !existing.region.is_same_region_as(request.region) {
            return Err(Violation::IncompleteFree {
                request: request.clone(),
                existing: existing.clone(),
            });
        }

        if existing.region.align != request.region.align {
            return Err(Violation::MisalignedFree {
                request: request.clone(),
                existing: existing.clone(),
            });
        }

        let (_, region) = entry.remove_entry();
        self.memory_used = self.memory_used.saturating_sub(region.region.size);
        Ok(())
    }

    /// Access all trailing regions (ones which have not been deallocated).
    pub fn trailing_regions(&self) -> Vec<Request> {
        self.regions.values().cloned().collect()
    }
}

/// Utility function to find overlapping regions.
fn find_region_overlaps(
    regions: &BTreeMap<Pointer, Request>,
    needle: Region,
) -> impl Iterator<Item = Request> + '_ {
    let head = regions
        .range(..=needle.ptr)
        .take_while(move |(_, r)| r.region.overlaps(needle));

    let tail = regions
        .range(needle.ptr..)
        .take_while(move |(_, r)| r.region.overlaps(needle));

    head.chain(tail).map(|(_, r)| r.clone())
}
