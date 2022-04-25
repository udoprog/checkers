# checkers

[<img alt="github" src="https://img.shields.io/badge/github-udoprog/checkers?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/checkers)
[<img alt="crates.io" src="https://img.shields.io/crates/v/checkers.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/checkers)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-checkers?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/checkers)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/udoprog/checkers/CI/main?style=for-the-badge" height="20">](https://github.com/udoprog/checkers/actions?query=branch%3Amain)

Checkers is a simple allocation sanitizer for Rust. It plugs in through the
[global allocator] and can sanity check your unsafe Rust during integration
testing. Since it plugs in through the global allocator it doesn't require any
additional dependencies and works for all platforms - but it is more limited in
what it can verify.

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

### Usage

Add `checkers` as a dev-dependency to your project:

```toml
checkers = "0.6.1"
```

Replace the global allocator in a [test file] and wrap tests you wish to
memory sanitise with `#[checkers::test]`:

```rust
#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[checkers::test]
fn test_allocations() {
    let _ = Box::into_raw(Box::new(42));
}
```

> Note that it's important that you write your test as an *integration test*
> by adding it to your `tests/` folder to isolate the use of the global
> allocator.

## Safety

With the default feature set, this library performs diagnostics which will
produce undefined behavior. Therefore, it is recommended that you only use
checkers for _testing_, and never in any production code.

If you want to avoid this, you'll have to disable the `realloc` and `zeroed`
features, but this will also produce less actionable diagnostics.

In a future release, this behavior will be changed to be opt-in through feature
flags instead of enabled by default.

## Features

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
  [`#[checkers::test]`][checkers-test].
* `backtrace` - Enables the capture and rendering of backtraces. If
  disabled, any fields containing backtraces will be `None`.

[realloc]: std::alloc::GlobalAlloc::realloc
[alloc_zeroed]: std::alloc::GlobalAlloc::alloc_zeroed
[#1]: https://github.com/udoprog/checkers/issues/1

## Examples

It is recommended that you use checkers for [integration tests], which by
default lives in the `./tests` directory. Each file in this directory will be
compiled as a separate program, so the use of the global allocator can be more
isolated.

We then use checkers by installing
[`checkers::Allocator`][checkers-allocator] as the global allocator, after
this we can make use of [`#[checkers::test]`][checkers-test] attribute macro
or the [`checkers::with`][checkers-with] function in our tests.

```rust
#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[checkers::test]
fn test_allocations() {
    let _ = Box::into_raw(Box::new(42));
}
```

The above would result in the following test output:

```
dangling region: 0x226e5784f30-0x226e5784f40 (size: 16, align: 8).
thread 'test_leak_box' panicked at 'allocation checks failed', tests\leaky_tests.rs:4:1
```

With [`checkers::with`][checkers-with], we can perform more detailed
diagnostics:

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

[test file]: https://doc.rust-lang.org/cargo/guide/project-layout.html
[checkers-allocator]: https://docs.rs/checkers/latest/checkers/struct.Allocator.html
[checkers-test]: https://docs.rs/checkers/latest/checkers/attr.test.html
[checkers-with]: https://docs.rs/checkers/latest/checkers/fn.with.html
[global allocator]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html
[integration tests]: https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests
[mismatched layout]: https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety
[see test]: https://github.com/udoprog/checkers/blob/master/tests/leaky_tests.rs

License: MIT/Apache-2.0
