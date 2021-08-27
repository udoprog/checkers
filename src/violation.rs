use std::fmt;

use crate::{Realloc, ReallocNull, Region, Request};

/// A single violation in the variants enforced by checkers.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Violation {
    /// A region produced by the allocator `requested`, overlaps with at least
    /// on `existing` allocation.
    ConflictingAlloc {
        /// The allocated region.
        request: Request,
        /// The existing region.
        existing: Request,
    },
    /// A region produced by the allocator `requested` was not zeroed as
    /// expected.
    NonZeroedAlloc {
        /// The allocated region.
        alloc: Request,
    },
    /// A region reallocatoed by the allocator was not copied appropriately.
    /// Meaning, the prefixing bytes between `free` and `alloc` do not match.
    NonCopiedRealloc {
        /// The reallocation.
        realloc: Realloc,
    },
    /// Allocator was asked to reallocate a null pointer.
    ReallocNull {
        /// The null relocation.
        realloc: ReallocNull,
    },
    /// A region produced by the allocator `requested` was not aligned
    /// appropriately.
    MisalignedAlloc {
        /// The allocated region.
        alloc: Request,
    },
    /// A freed region `requested` only freed part of at least one other region
    /// `existing`.
    IncompleteFree {
        /// The freed region.
        request: Request,
        /// The existing region.
        existing: Request,
    },
    /// A freed region `requested` provided the wrong alignment metadata.
    /// See [std::alloc::Layout::align].
    MisalignedFree {
        /// The freed region.
        request: Request,
        /// The existing region.
        existing: Request,
    },
    /// A freed region `requested` was not allocated at the time it was freed.
    MissingFree {
        /// The freed region.
        request: Request,
    },
    /// A `region` was leaked. In that it was allocated but never freed.
    Leaked {
        /// The leaked region.
        alloc: Request,
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
    /// # use checkers::{Request, Region, Violation};
    /// let alloc = Request::without_backtrace(Region::new(42.into(), 20, 4));
    /// let violation = Violation::Leaked { alloc };
    /// assert!(violation.is_leaked_with(|r| r.size == 20 && r.align == 4));
    ///
    /// let alloc = Request::without_backtrace(Region::new(10.into(), 10, 1));
    /// let violation = Violation::MisalignedAlloc { alloc };
    /// assert!(!violation.is_leaked_with(|r| true));
    /// ```
    pub fn is_leaked_with<F>(&self, f: F) -> bool
    where
        F: FnOnce(Region) -> bool,
    {
        match self {
            Self::Leaked { alloc } => f(alloc.region),
            _ => false,
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingAlloc { request, existing } => {
                write!(
                    f,
                    "Requested allocation ({}) overlaps with existing ({})",
                    request.region, existing.region
                )?;

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Allocation Backtrace: {:?}", bt)?;
                }

                if let Some(bt) = &existing.backtrace {
                    writeln!(f)?;
                    write!(f, "Existing Allocation Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::NonZeroedAlloc { alloc } => {
                write!(
                    f,
                    "Requested allocation ({}) was not zerod by the allocator",
                    alloc.region
                )?;

                if let Some(bt) = &alloc.backtrace {
                    writeln!(f)?;
                    write!(f, "Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::NonCopiedRealloc { realloc } => {
                write!(
                    f,
                    "Relocating from ({}) to ({}) did not correctly copy the prefixing bytes",
                    realloc.free, realloc.alloc,
                )?;

                if let Some(bt) = &realloc.backtrace {
                    writeln!(f)?;
                    write!(f, "Reallocation Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::ReallocNull { realloc } => {
                write!(f, "Tried to reallocate null pointer")?;

                if let Some(bt) = &realloc.backtrace {
                    writeln!(f)?;
                    write!(f, "Reallocation Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::MisalignedAlloc { alloc } => {
                write!(f, "Allocated region ({}) is misaligned.", alloc.region)?;

                if let Some(bt) = &alloc.backtrace {
                    writeln!(f)?;
                    write!(f, "Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::IncompleteFree { request, existing } => {
                write!(
                    f,
                    "Freed ({}) only part of existing region ({})",
                    request.region, existing.region
                )?;

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Requested Backtrace: {:?}", bt)?;
                }

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Existing Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::MisalignedFree { request, existing } => {
                write!(
                    f,
                    "Freed region ({}) has different alignment from existing ({})",
                    request.region, existing.region
                )?;

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Requested Backtrace: {:?}", bt)?;
                }

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Existing Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::MissingFree { request } => {
                write!(f, "Freed missing region ({})", request.region)?;

                if let Some(bt) = &request.backtrace {
                    writeln!(f)?;
                    write!(f, "Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
            Self::Leaked { alloc } => {
                write!(f, "Dangling region ({})", alloc.region)?;

                if let Some(bt) = &alloc.backtrace {
                    writeln!(f)?;
                    write!(f, "Backtrace: {:?}", bt)?;
                }

                Ok(())
            }
        }
    }
}
