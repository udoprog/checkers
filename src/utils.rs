/// Test if all bytes are zeroed.
///
/// # Safety
///
/// `ptr` needs to be non-null and initialized to the length specified in `len`.
#[cfg(feature = "zeroed")]
pub(crate) unsafe fn is_zeroed_ptr(ptr: *const u8, len: usize) -> bool {
    debug_assert!(!ptr.is_null());
    std::slice::from_raw_parts(ptr, len).iter().all(|b| *b == 0)
}

/// Hash the given collection of bytes.
///
/// # Safety
///
/// `ptr` needs to be non-null and initialized to the length specified in `len`.
#[cfg(feature = "realloc")]
pub(crate) unsafe fn hash_ptr(ptr: *const u8, len: usize) -> impl PartialEq + Eq {
    debug_assert!(!ptr.is_null());
    fxhash::hash64(std::slice::from_raw_parts(ptr, len))
}
