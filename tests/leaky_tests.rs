#[global_allocator]
static CHECKED: checkers::Allocator = checkers::Allocator;

#[checkers::test]
#[should_panic]
fn test_leak_box() {
    let _ = Box::into_raw(Box::new(0u128));
}

#[checkers::test]
fn test_non_leak_box() {
    let b = Box::into_raw(Box::new(0u128));
    let _ = unsafe { Box::from_raw(b) };
}

fn verify_test_custom_verify(state: &mut checkers::State) {
    let mut violations = Vec::new();
    state.validate(&mut violations);
    assert_eq!(1, violations.len());
    assert!(violations[0].is_leaked_with(|region| region.size == 20 && region.align == 4));
}

#[checkers::test(verify = "verify_test_custom_verify")]
fn test_custom_verify() {
    let _ = Box::into_raw(vec![1, 2, 3, 4, 5].into_boxed_slice());
}
