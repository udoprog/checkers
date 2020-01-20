use std::fmt;

use crate::Region;

/// A single violation in the variants enforced by checkers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A region produced by the allocator `requested`, overlaps with at least
    /// on `existing` allocation.
    ConflictingAlloc {
        /// The allocated region.
        requested: Region,
        /// The existing region.
        existing: Region,
    },
    /// A region produced by the allocator `requested` was not zeroed as
    /// expected.
    NonZeroedAlloc {
        /// The allocated region.
        requested: Region,
    },
    /// A region reallocatoed by the allocator was not copied appropriately.
    /// Meaning, the prefixing bytes between `free` and `alloc` do not match.
    NonCopiedRealloc {
        /// The freed region.
        free: Region,
        /// The allocated region.
        alloc: Region,
    },
    /// Allocator was asked to reallocate a null pointer.
    ReallocNull {},
    /// A region produced by the allocator `requested` was not aligned
    /// appropriately.
    MisalignedAlloc {
        /// The allocated region.
        requested: Region,
    },
    /// A freed region `requested` only freed part of at least one other region
    /// `existing`.
    IncompleteFree {
        /// The freed region.
        requested: Region,
        /// The existing region.
        existing: Region,
    },
    /// A freed region `requested` provided the wrong alignment metadata.
    /// See [std::alloc::Layout::align].
    MisalignedFree {
        /// The freed region.
        requested: Region,
        /// The existing region.
        existing: Region,
    },
    /// A freed region `requested` was not allocated at the time it was freed.
    MissingFree {
        /// The freed region.
        requested: Region,
    },
    /// A `region` was leaked. In that it was allocated but never freed.
    Leaked {
        /// The leaked region.
        region: Region,
    },
}

/// A single violation to the virtual memory model of checkers.
impl Violation {
    /// Test that this violation refers to a dangling region and that it matches
    /// the given predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use checkers::{Region, Violation};
    /// let violation = Violation::Leaked {
    ///     region: Region::new(42.into(), 20, 4),
    /// };
    /// assert!(violation.is_leaked_with(|r| r.size == 20 && r.align == 4));
    ///
    /// let requested = Region::new(10.into(), 10, 1);
    /// let violation = Violation::MisalignedAlloc { requested };
    /// assert!(!violation.is_leaked_with(|r| true));
    /// ```
    pub fn is_leaked_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match *self {
            Self::Leaked { region } => f(region),
            _ => false,
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingAlloc {
                requested,
                existing,
            } => write!(
                fmt,
                "Requested allocation ({}) overlaps with existing ({})",
                requested, existing
            ),
            Self::NonZeroedAlloc { requested } => write!(
                fmt,
                "Requested allocation ({}) was not zerod by the allocator",
                requested
            ),
            Self::NonCopiedRealloc { free, alloc } => write!(
                fmt,
                "Relocating from ({}) to ({}) did not correctly copy the prefixing bytes",
                free, alloc,
            ),
            Self::ReallocNull {} => write!(fmt, "Tried to reallocate null pointer"),
            Self::MisalignedAlloc { requested } => {
                write!(fmt, "Allocated region ({}) is misaligned.", requested)
            }
            Self::IncompleteFree {
                requested,
                existing,
            } => write!(
                fmt,
                "Freed ({}) only part of existing region ({})",
                requested, existing
            ),
            Self::MisalignedFree {
                requested,
                existing,
            } => write!(
                fmt,
                "Freed region ({}) has different alignment from existing ({})",
                requested, existing
            ),
            Self::MissingFree { requested } => write!(fmt, "Freed missing region ({})", requested),
            Self::Leaked { region } => write!(fmt, "Dangling region ({})", region),
        }
    }
}
