# checkers

[![Documentation](https://docs.rs/checkers/badge.svg)](https://docs.rs/checkers)
[![Crates](https://img.shields.io/crates/v/checkers.svg)](https://crates.io/crates/checkers)
[![Actions Status](https://github.com/udoprog/checkers/workflows/Rust/badge.svg)](https://github.com/udoprog/checkers/actions)

Checkers is a simple allocation checker for Rust that runs purely inside of Rust.

# Examples

You use checkers by installing it's allocator, then making use of either the
`#[checkers::test]` or `checkers::with!` macros.

```rust
#[global_allocator]
static CHECKED: checkers::Allocator = checkers::Allocator;

#[checkers::test]
fn test_allocations() {
    let _ = Box::into_raw(Box::new(42));
}

#[test]
fn test_manually() {
    checkers::with!(|| {
        let _ = Box::into_raw(Box::new(42));
    });
}
```