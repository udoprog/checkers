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

# Safety

With the default feature set, this library performs diagnostics which will
produce undefined behavior. Therefore, it is recommended that you only use
checkers for _testing_, and never in any production code.

If you want to avoid this, you'll have to disable the `realloc` and `zeroed`
features, but this will also produce less actionable diagnostics.

In a future release, this behavior will be changed to be opt-in through feature
flags instead of enabled by default.

# Features

The following are features available, that changes how checkers work.

* `realloc` - Enabling this feature causes checker to verify that a [realloc]
  operation is correctly implemented. That bytes from the old region were
  faithfully transferred to the new, resized one.
  Since this can have a rather significant performance impact, it can be
  disabled.
  Note that this will produce undefined behavior ([#1]) by reading uninitialized
  memory, and should only be enabled to provide diagnostics on a best-effort
  basis.
* `zeroed` - Enabling this feature causes checkers to verify that a call to
  [alloc_zeroed] produces a region where all bytes are _set_ to zero.
  Note that if the underlying allocator is badly implemented this will produce
  undefined behavior ([#1]) since it could read uninitialized memory.
* `macros` - Enables dependencies and re-exports of macros, like
  [`#[checkers::test]`](attr.test.html).

[realloc]: https://doc.rust-lang.org/nightly/core/alloc/trait.GlobalAlloc.html#method.realloc
[alloc_zeroed]: https://doc.rust-lang.org/nightly/core/alloc/trait.GlobalAlloc.html#method.alloc_zeroed
[#1]: https://github.com/udoprog/checkers/issues/1

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
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

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
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[test]
fn test_event_inspection() {
    let snapshot = checkers::with(|| {
        let _ = vec![1, 2, 3, 4];
    });

    assert_eq!(2, snapshot.events.len());
    assert!(snapshot.events[0].is_alloc_with(|r| r.size >= 16));
    assert!(snapshot.events[1].is_free_with(|a| a.size >= 16));
    assert_eq!(1, snapshot.events.allocs());
    assert_eq!(1, snapshot.events.frees());
    assert!(snapshot.events.max_memory_used().unwrap() >= 16);
}
```
