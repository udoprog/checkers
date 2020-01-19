use checkers::{Event, Events, Region};

#[test]
fn events_test() {
    let mut events = Events::new();

    for _ in 0..800 {
        events.push(Event::Alloc(Region::new(10.into(), 10, 1)));
    }

    assert_eq!(800, events.as_slice().len());
}
