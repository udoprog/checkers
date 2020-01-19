/// Test if all bytes are zeroed.
pub(crate) fn is_zeroed(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| *b == 0)
}

/// Hash the given collection of bytes.
#[cfg(feature = "realloc")]
pub(crate) unsafe fn hash_ptr(ptr: *const u8, len: usize) -> impl PartialEq + Eq {
    use std::slice;
    fxhash::hash64(slice::from_raw_parts(ptr, len))
}
