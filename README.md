# checkers

[![Documentation](https://docs.rs/checkers/badge.svg)](https://docs.rs/checkers)
[![Crates](https://img.shields.io/crates/v/checkers.svg)](https://crates.io/crates/checkers)
[![Actions Status](https://github.com/udoprog/checkers/workflows/Rust/badge.svg)](https://github.com/udoprog/checkers/actions)

Checkers is a simple allocation checker for Rust that runs purely inside of Rust.

# Examples

You use checkers by installing it's allocator, then making use of
`checkers::with!`.

```rust
#[global_allocator]
static CHECKED: checkers::Allocator = checkers::Allocator;

#[test]
fn test_allocations() {
    checkers::with!(|| {
        let mut bytes = vec![10, 20, 30];
        bytes.truncate(2);
    });
}
```