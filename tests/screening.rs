//! Arranging with a screening model evaluates only the moves it ranks
//! first, and arranges as before when it may keep them all.
#![cfg(feature = "screening")]

use nicevg::screening::{Screen, arrange_screened};

/// One tree: candidates with no earlier gain (missing) rank below those
/// that improved before.
const MODEL: &str = r#"{"learner": {
    "feature_names": ["candidates", "prior_gain~", "issues_here_rank"],
    "learner_model_param": {"base_score": "[5E-1]"},
    "objective": {"name": "binary:logistic"},
    "gradient_booster": {"model": {"trees": [{
        "left_children": [1, -1, -1], "right_children": [2, -1, -1],
        "split_indices": [1, 0, 0], "split_conditions": [0.0, 1.0, -1.0],
        "default_left": [0, 0, 0]
    }]}}
}}"#;

#[test]
fn keeping_every_candidate_arranges_as_arrange_does() {
    // Given a diagram, and a model allowed to keep more candidates than any step has
    let svg = include_str!("fixtures/order-checkout-dfd.svg");
    let screen = Screen::from_xgboost_json(MODEL, usize::MAX).expect("the model reads");

    // When it is arranged with the model and without
    let screened = arrange_screened(svg, &screen).expect("arranges");
    let plain = nicevg::arrange(svg).expect("arranges");

    // Then both come out the same
    assert_eq!(screened.svg, plain.svg);
}

#[test]
fn keeping_one_candidate_a_step_still_arranges() {
    // Given a model allowed one candidate a step
    let svg = include_str!("fixtures/order-checkout-dfd.svg");
    let screen = Screen::from_xgboost_json(MODEL, 1).expect("the model reads");

    // When the diagram is arranged with it
    let screened = arrange_screened(svg, &screen).expect("arranges");

    // Then every node is still there, and it has no more issues than fixing alone leaves,
    // as no move arrange takes adds one
    let fixed = nicevg::fix(svg).expect("fixes");
    assert_eq!(screened.report.diagram.nodes.len(), fixed.report.diagram.nodes.len());
    assert!(screened.report.issues.len() <= fixed.report.issues.len());
}
