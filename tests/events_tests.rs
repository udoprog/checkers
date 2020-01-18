use checkers::{Event, Events};

#[test]
fn events_test() {
    let mut events = Events::new();

    for _ in 0..800 {
        events.push(Event::Empty);
    }

    assert_eq!(800, events.as_slice().len());
}
