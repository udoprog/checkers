# checkers

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