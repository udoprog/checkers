pub(crate) use ::backtrace::Backtrace;

pub(crate) fn capture() -> Option<::backtrace::Backtrace> {
    Some(::backtrace::Backtrace::new())
}
