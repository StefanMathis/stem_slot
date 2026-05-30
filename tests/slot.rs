use std::f64::consts::{FRAC_PI_2, PI, TAU};

use indoc::indoc;
use stem_slot::prelude::*;
use stem_slot::slot::*;

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

#[test]
fn test_deserialize_bottom_with_width_and_height() {
    let data = indoc! {"
        ---
        bottom_width: 1.0 m
        bottom_side_width: 3.0 m
        bottom_height: 1.0 m
        slot_angle: 10.0 deg
        "};
    let bottom_angle: BottomAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(bottom_angle.value(), 0.75 * PI, epsilon = 1e-15);

    let data = indoc! {"
        ---
        10.0 deg
        "};
    let bottom_angle: BottomAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(bottom_angle.value(), TAU / 36.0, epsilon = 1e-15);

    let data = indoc! {"
        ---
        1.0
        "};
    let bottom_angle: BottomAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(bottom_angle.value(), 1.0, epsilon = 1e-15);
}

#[test]
fn test_deserialize_top_with_width_and_height() {
    let data = indoc! {"
        ---
        top_width: 1.0
        top_side_width: 3.0
        top_height: 1.0
        slot_angle: 10.0 deg
        "};
    let top_angle: TopAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(top_angle.value(), 0.75 * PI, epsilon = 1e-15);

    let data = indoc! {"
        ---
        10.0 deg
        "};
    let top_angle: TopAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(top_angle.value(), TAU / 36.0, epsilon = 1e-15);

    let data = indoc! {"
        ---
        1.0
        "};
    let top_angle: TopAngle = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(top_angle.value(), 1.0, epsilon = 1e-15);
}

#[test]
fn test_angle_bottom_from_width_height() {
    let slot_angle = TAU / 36.0; // 10°

    // Case: Vertical slope
    approx::assert_abs_diff_eq!(
        BottomAngle::FromWidthAndHeight {
            bottom_width: Length::new::<millimeter>(1.0),
            bottom_side_width: Length::new::<millimeter>(1.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        FRAC_PI_2,
        epsilon = 1e-6
    );

    // Case: No slope
    approx::assert_abs_diff_eq!(
        BottomAngle::FromWidthAndHeight {
            bottom_width: Length::new::<millimeter>(1.0),
            bottom_side_width: Length::new::<millimeter>(1.0),
            bottom_height: Length::new::<millimeter>(0.0),
            slot_angle
        }
        .value(),
        FRAC_PI_2 - 0.5 * slot_angle,
        epsilon = 1e-6
    );

    // Case: gentle slope
    approx::assert_abs_diff_eq!(
        2.677945,
        BottomAngle::FromWidthAndHeight {
            bottom_width: Length::new::<millimeter>(1.0),
            bottom_side_width: Length::new::<millimeter>(3.0),
            bottom_height: Length::new::<millimeter>(0.5),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 45°
    approx::assert_abs_diff_eq!(
        0.75 * PI,
        BottomAngle::FromWidthAndHeight {
            bottom_width: Length::new::<millimeter>(1.0),
            bottom_side_width: Length::new::<millimeter>(3.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: steep slope
    approx::assert_abs_diff_eq!(
        2.03444393,
        BottomAngle::FromWidthAndHeight {
            bottom_width: Length::new::<millimeter>(1.0),
            bottom_side_width: Length::new::<millimeter>(2.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );
}

#[test]
fn test_top_angle_from_width_height() {
    let slot_angle = TAU / 36.0; // 10°

    // Case: Vertical slope (bottom_width = bottom_side_width)
    approx::assert_abs_diff_eq!(
        FRAC_PI_2,
        TopAngle::FromWidthAndHeight {
            top_width: Length::new::<millimeter>(1.0),
            top_side_width: Length::new::<millimeter>(1.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: No slope (bottom_width = bottom_side_width)
    approx::assert_abs_diff_eq!(
        FRAC_PI_2 + 0.5 * slot_angle,
        TopAngle::FromWidthAndHeight {
            top_width: Length::new::<millimeter>(1.0),
            top_side_width: Length::new::<millimeter>(1.0),
            top_height: Length::new::<millimeter>(0.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        2.677945,
        TopAngle::FromWidthAndHeight {
            top_width: Length::new::<millimeter>(1.0),
            top_side_width: Length::new::<millimeter>(3.0),
            top_height: Length::new::<millimeter>(0.5),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 45°
    approx::assert_abs_diff_eq!(
        0.75 * PI,
        TopAngle::FromWidthAndHeight {
            top_width: Length::new::<millimeter>(1.0),
            top_side_width: Length::new::<millimeter>(3.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        2.034443,
        TopAngle::FromWidthAndHeight {
            top_width: Length::new::<millimeter>(1.0),
            top_side_width: Length::new::<millimeter>(2.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );
}
