# checkers

[![Documentation](https://docs.rs/checkers/badge.svg)](https://docs.rs/checkers)
[![Crates](https://img.shields.io/crates/v/checkers.svg)](https://crates.io/crates/checkers)
[![Actions Status](https://github.com/udoprog/checkers/workflows/Rust/badge.svg)](https://github.com/udoprog/checkers/actions)

Checkers is a simple allocation checker for Rust. It plugs in through the
[global allocator] API and can sanity check your unsafe Rust during integration
testing.

[global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html

It can check for the following things:
* Double-frees.
* Attempts to free regions which are not allocated.
* Underlying allocator producting regions not adhering to the requested layout.
  Namely size and alignment.
* Other arbitrary user-defined conditions ([see test]).

What it can't do:
* Test multithreaded code. Since the allocator is global, it is difficult to
  scope the state for each test case.

[see test]: tests/leaky_tests.rs

# Examples

You use checkers by installing `checkers::Allocator` as your allocator, then
making use of either the `#[checkers::test]` or `checkers::with!` macros.

```rust
#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator;

#[checkers::test]
fn test_allocations() {
    let _ = Box::into_raw(Box::new(42));
}
```

The above would result in the following test output:

```text
dangling region: 0x226e5784f30-0x226e5784f40 (size: 16, align: 8).
thread 'test_leak_box' panicked at 'allocation checks failed', tests\leaky_tests.rs:4:1
```