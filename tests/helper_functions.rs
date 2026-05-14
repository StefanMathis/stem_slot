use stem_slot::slot::semi_regular_polygon_side_length;

#[test]
fn test_semi_regular_polygon_side_length() {
    // This is actually a regular polygon with 12 sides in total.
    let first_side = 1.0;
    let second_side = semi_regular_polygon_side_length(
        first_side,
        first_side * (2.0f64 + 3.0f64.sqrt()).sqrt(),
        12,
    )
    .unwrap();
    approx::assert_abs_diff_eq!(first_side, second_side);

    // Now for an irregular polygon
    let first_side = 1.0;
    let second_side = semi_regular_polygon_side_length(first_side, 2.0, 12).unwrap();
    approx::assert_abs_diff_eq!(1.070466, second_side, epsilon = 1e-6);

    // And now some failed attempts
    assert!(semi_regular_polygon_side_length(-1.0, 2.0, 12).is_none());
    assert!(semi_regular_polygon_side_length(1.0, -2.0, 12).is_none());
    assert!(semi_regular_polygon_side_length(1.0, 2.0, 11).is_none());
}
