use cairo_viewport::*;
use indoc::indoc;
use planar_geo::prelude::*;
use std::f64::consts::PI;
use stem_slot::{open_trapezoid::*, prelude::*};

fn compare_to_reference<P: AsRef<std::path::Path>>(
    drawables: &[DrawableCow<'_>],
    path: P,
    view: Option<Viewport>,
) {
    let view = view.unwrap_or(
        Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap(),
    );
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, move |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;
            for drawable in drawables.iter() {
                drawable.draw(cr)?;
            }
            return Ok(());
        });
    };
    assert!(compare_or_create(path, callback, 0.98).is_ok());
}

#[test]
fn test_shrunk_fillet_radii() {
    let builder = OpenTrapezoidWithBottomAngleBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_angle: BottomAngle::FromBottomSideAngle {
            bottom_side_angle: 120.0 * PI / 180.0,
            slot_angle: 10.0 * PI / 180.0,
        },
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(20.0),
        bottom_side_radius: Length::new::<millimeter>(0.5),
        consider_tooth_tip_leakage: true,
    };
    let slot = OpenTrapezoidSlot::try_from(builder).unwrap();
    approx::assert_abs_diff_eq!(
        slot.bottom_radius().get::<millimeter>(),
        5.4681,
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        slot.bottom_side_radius().get::<millimeter>(),
        0.5,
        epsilon = 1e-6
    );
}

#[test]
fn test_angle_bottom() {
    let builder = OpenTrapezoidWithBottomAngleBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_angle: BottomAngle::FromBottomSideAngle {
            bottom_side_angle: 120.0 * PI / 180.0,
            slot_angle: 10.0 * PI / 180.0,
        },
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    };
    let slot = OpenTrapezoidSlot::try_from(builder).unwrap();

    approx::assert_abs_diff_eq!(slot.bottom_side_angle(), 120.0 * PI / 180.0, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(slot.outline().length(), 0.0465666, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(
        slot.area().get::<square_millimeter>(),
        132.78895,
        epsilon = 1e-3
    );

    // Check some geometric parameters
    approx::assert_abs_diff_eq!(
        122.4,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-1
    );
    approx::assert_abs_diff_eq!(
        132.7,
        slot.area().get::<square_millimeter>(),
        epsilon = 1e-1
    );
    approx::assert_abs_diff_eq!(
        16.8455,
        slot.side_height().get::<millimeter>(),
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        1.154,
        slot.bottom_height().get::<millimeter>(),
        epsilon = 1e-3
    );

    // Check the slot leakage coefficients
    approx::assert_abs_diff_eq!(
        1.08016,
        slot.self_inductance_leakage_coefficient(0, &CoilLayout::Single),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(0.4, slot.leakage_coefficient_opening(), epsilon = 0.001);
    approx::assert_abs_diff_eq!(
        -0.06939,
        slot.leakage_coefficient_tooth_tip(Length::new::<millimeter>(1.0)),
        epsilon = 0.001
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::SingleFilled).as_slice(),
        "tests/img/open_trapezoid_slot_angle_bottom.png",
        None,
    );
}

#[test]
fn test_different_layers() {
    let builder = OpenTrapezoidWithBottomAngleBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_angle: BottomAngle::FromBottomSideAngle {
            bottom_side_angle: 120.0 * PI / 180.0,
            slot_angle: 10.0 * PI / 180.0,
        },
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    };
    let slot = OpenTrapezoidSlot::try_from(builder).unwrap();

    {
        // Horizontal
        let drawables = slot.drawables(&CoilLayout::DoubleVertical);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/open_trapezoid_slot_vert.png",
            None,
        );
        compare_to_reference(
            &[drawables[0].clone()],
            "tests/img/open_trapezoid_slot_vert_layer_1.png",
            Some(Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap()),
        );
        compare_to_reference(
            &[drawables[1].clone()],
            "tests/img/open_trapezoid_slot_vert_layer_2.png",
            Some(Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap()),
        );
    }
    {
        // Vertical
        let drawables = slot.drawables(&CoilLayout::DoubleHorizontal);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/open_trapezoid_slot_hori.png",
            None,
        );
        compare_to_reference(
            &[drawables[0].clone()],
            "tests/img/open_trapezoid_slot_hori_layer_1.png",
            Some(Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap()),
        );
        compare_to_reference(
            &[drawables[1].clone()],
            "tests/img/open_trapezoid_slot_hori_layer_2.png",
            Some(Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap()),
        );
    }
}

#[test]
fn test_open_slot_bottom_height() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidWithBottomHeightBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_height: Length::new::<millimeter>(1.154),
        bottom_width: Length::new::<millimeter>(5.0),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Check some geometric parameters
    approx::assert_abs_diff_eq!(
        16.8455,
        slot.side_height().get::<millimeter>(),
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        122.4,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-1
    );

    // Image comparison
    let drawables = slot.drawables(&CoilLayout::DoubleHorizontal);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_hori.png",
        None,
    );
}

#[test]
fn test_open_slot_bottom_slope_width() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidWithBottomSideWidthBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_side_width: Length::new::<millimeter>(8.298),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Check some geometric parameters
    approx::assert_abs_diff_eq!(
        122.4,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-1
    );

    // Image comparison
    let drawables = slot.drawables(&CoilLayout::DoubleVertical);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_vert.png",
        None,
    );
}

#[test]
fn test_open_slot_side_height_bugfix() {
    let bottom_radius = Length::new::<millimeter>(2.0);
    let slot_angle = PI / 18.0;
    let bottom_width = Length::new::<millimeter>(9.21);
    let slot: OpenTrapezoidSlot = OpenTrapezoidBuilder {
        bottom_width,
        opening_width: bottom_width
            - Length::new::<millimeter>(2.0 * 17.75) * (slot_angle / 2.0).sin(),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle,
        bottom_radius,
        bottom_side_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(&CoilLayout::DoubleVertical);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_bugfix.png",
        None,
    );
}

#[test]
fn test_test_from_rotary_core() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidFromToothWidthRotBuilder {
        tooth_width: Length::new::<millimeter>(10.0),
        air_gap_radius: Length::new::<millimeter>(37.5),
        yoke_radius: Length::new::<millimeter>(50.0),
        slots: 12,
        opening_width: Length::new::<millimeter>(9.6),
        height: Length::new::<millimeter>(20.0),
        bottom_height: Length::new::<millimeter>(0.0),
        opening_height: Length::new::<millimeter>(1.0),
        bottom_radius: Length::new::<millimeter>(0.0),
        bottom_side_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(&CoilLayout::SingleFilled);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_from_rotative_core.png",
        None,
    );
}

#[test]
fn test_multilayer_vertical() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidWithBottomSideWidthBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_side_width: Length::new::<millimeter>(8.298),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let drawables = slot.drawables(&CoilLayout::MultiVertical(1));
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_1_vertical.png",
        None,
    );

    let drawables = slot.drawables(&CoilLayout::MultiVertical(2));
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_2_vertical.png",
        None,
    );

    let drawables = slot.drawables(&CoilLayout::MultiVertical(3));
    let view = Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap();
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_3_vertical.png",
        Some(view),
    );

    compare_to_reference(
        &[drawables[0].clone()],
        "tests/img/open_trapezoid_slot_3_vertical_layer_1.png",
        Some(view),
    );

    compare_to_reference(
        &[drawables[1].clone()],
        "tests/img/open_trapezoid_slot_3_vertical_layer_2.png",
        Some(view),
    );

    compare_to_reference(
        &[drawables[2].clone()],
        "tests/img/open_trapezoid_slot_3_vertical_layer_3.png",
        Some(view),
    );

    let drawables = slot.drawables(&CoilLayout::MultiVertical(4));
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_4_vertical.png",
        None,
    );
}

#[test]
fn test_serialize_and_deserialize() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidWithBottomSideWidthBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        bottom_side_width: Length::new::<millimeter>(8.298),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        bottom_side_radius: Length::new::<millimeter>(1.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();
    let serialized = serde_yaml::to_string(&slot).expect("can be serialized");
    let slot_de: OpenTrapezoidSlot =
        serde_yaml::from_str(&serialized).expect("can be deserialized");
    approx::assert_abs_diff_eq!(
        slot.area().get::<square_millimeter>(),
        slot_de.area().get::<square_millimeter>(),
        epsilon = DEFAULT_EPSILON
    );
}

#[test]
fn test_deserialize() {
    {
        let yaml = indoc! {"
                ---
                opening_width: 9.6 mm
                height: 20 mm
                opening_height: 0.0 mm
                slot_angle: 30.0 deg
                bottom_radius: 1 mm
                consider_tooth_tip_leakage: false
                "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(&CoilLayout::DoubleHorizontal);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/open_trapezoid_slot_MEAS-Servo.png",
            None,
        );
    }
    {
        let yaml = indoc! {"
                        ---
                        opening_width: 9.6 mm
                        height: 20 mm
                        opening_height: 1.0 mm
                        slot_angle: 30.0 deg
                        bottom_radius: 1 mm
                        consider_tooth_tip_leakage: false
                        "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(&CoilLayout::DoubleHorizontal);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/open_trapezoid_opening_height_1mm.png",
            None,
        );
    }
    {
        let yaml = indoc! {"
                        ---
                        opening_width: 9.6 mm
                        height: 20 mm
                        opening_height: 5.0 mm
                        slot_angle: 30.0 deg
                        bottom_radius: 1 mm
                        consider_tooth_tip_leakage: false
                        "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(&CoilLayout::DoubleHorizontal);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/open_trapezoid_opening_height_5mm.png",
            None,
        );
    }
}
