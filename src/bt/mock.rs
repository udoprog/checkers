/// Mock backtrace implementation.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Backtrace(());

impl Backtrace {
    pub(crate) fn new() -> Self {
        Self(())
    }
}
