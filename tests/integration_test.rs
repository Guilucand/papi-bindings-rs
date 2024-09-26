use papi_bindings::counter::Counter;
use papi_bindings::events_set::EventsSet;
use papi_bindings::{initialize, terminate};

#[test]
fn test_fib() {
    initialize(true).unwrap();

    let counters = vec![
        Counter::from_name("ix86arch::INSTRUCTION_RETIRED").unwrap(),
        Counter::from_name("ix86arch::MISPREDICTED_BRANCH_RETIRED").unwrap(),
    ];

    let mut event_set = EventsSet::new(&counters).unwrap();

    for fv in 1..15 {
        event_set.start().unwrap();
        let x = fib(fv);
        let counters = event_set.stop().unwrap();
        println!(
            "Computed fib({}) = {} in {} instructions [mispredicted: {}].",
            fv, x, counters[0], counters[1]
        );
    }
    drop(counters);
    drop(event_set);
    terminate()
}

fn fib(n: isize) -> isize {
    if n < 2 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}
