//! Recording arrange's candidates also logs each step, and replaying the
//! log makes the same records without arranging again.
#![cfg(feature = "trace-candidates")]

#[test]
fn replaying_the_log_makes_the_records_arranging_made() {
    // Given a diagram arranged over several steps, its candidates recorded
    let svg = include_str!("fixtures/order-checkout-dfd.svg");
    nicevg::arrange(svg).expect("the fixture arranges");
    let records = nicevg::candidate_trace::take();
    let log = nicevg::candidate_trace::take_log();
    assert!(log["steps"].as_array().is_some_and(|steps| steps.len() > 1), "{log}");

    // When its log is replayed
    let replayed = nicevg::candidate_trace::replay(&log).expect("the log replays");

    // Then every record comes back as arranging made it
    assert_eq!(replayed, records);
}
