pub(crate) fn capture() -> Option<Backtrace> {
    None
}

/// Mock backtrace implementation.
#[derive(Debug, Clone, Copy)]
pub struct Backtrace(());
