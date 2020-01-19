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
* Attempts to free only part of regions which are allocated.
* Attempts to free a region with a [mismatching layout].
* Underlying allocator producting regions not adhering to the requested layout.
  Namely size and alignment.
* Other arbitrary user-defined conditions ([see test]).

What it can't do:
* Test multithreaded code. Since the allocator is global, it is difficult to
  scope the state for each test case.

[mismatching layout]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety
[see test]: tests/leaky_tests.rs

# Examples

You use checkers by installing `checkers::Allocator` as your allocator, then
making use of either the `#[checkers::test]` macro or the `checkers::with`
function.

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

With `checkers::with`, we can perform more detailed diagnostics:

```rust
#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator;

let snapshot = checkers::with(|| {
    let _ = vec![1, 2, 3, 4];
});

assert_eq!(2, snapshot.events.len());
assert!(snapshot.events[0].is_allocation_with(|r| r.size >= 16));
assert!(snapshot.events[1].is_deallocation_with(|a| a.size >= 16));
assert_eq!(1, snapshot.events.allocations());
assert_eq!(1, snapshot.events.deallocations());
assert!(snapshot.events.max_memory_used().unwrap() >= 16);
```