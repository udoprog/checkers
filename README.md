# checkers

[![Documentation](https://docs.rs/checkers/badge.svg)](https://docs.rs/checkers)
[![Crates](https://img.shields.io/crates/v/checkers.svg)](https://crates.io/crates/checkers)
[![Actions Status](https://github.com/udoprog/checkers/workflows/Rust/badge.svg)](https://github.com/udoprog/checkers/actions)

Checkers is a simple allocation sanitizer for Rust. It plugs in through the
[global allocator] and can sanity check your unsafe Rust during integration
testing. Since it plugs in through the global allocator it doesn't require any
additional dependencies and works for all platforms - but it is more limited in
what it can verify.

[global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html

It can check for the following things:
* Double-frees.
* Memory leaks.
* Freeing regions which are not allocated.
* Freeing only part of regions which are allocated.
* Freeing a region with a [mismatched layout].
* That the underlying allocator produces regions adhering to the requested
  layout. Namely size and alignment.
* Detailed information on memory usage.
* Other user-defined conditions ([see test]).

What it can't do:
* Test multithreaded code. Since the allocator is global, it is difficult to
  scope the state for each test case.
* Detect out-of-bounds accesses.

[mismatched layout]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety
[see test]: tests/leaky_tests.rs

# Examples

It is recommended that you use checkers for [integration tests], which by
default lives in the `./tests` directory. Each file in this directory will be
compiled as a separate program, so the use of the global allocator can be more
isolated.

[integration tests]: https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests

We then use checkers by installing `checkers::Allocator` as the global
allocator, after this we can make use of [`#[checkers::test]`](https://docs.rs/checkers/latest/checkers/attr.test.html) attribute macro or
the [`checkers::with`](https://docs.rs/checkers/latest/checkers/fn.with.html) function in our tests.

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

#[test]
fn test_event_inspection() {
    let snapshot = checkers::with(|| {
        let _ = vec![1, 2, 3, 4];
    });

    assert_eq!(2, snapshot.events.len());
    assert!(snapshot.events[0].is_allocation_with(|r| r.size >= 16));
    assert!(snapshot.events[1].is_deallocation_with(|a| a.size >= 16));
    assert_eq!(1, snapshot.events.allocations());
    assert_eq!(1, snapshot.events.deallocations());
    assert!(snapshot.events.max_memory_used().unwrap() >= 16);
}
```
