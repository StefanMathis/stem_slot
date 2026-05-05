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
    assert!(compare_or_create(path, callback, 0.99).is_ok());
}

#[test]
fn test_angle_bottom() {
    let builder = OpenTrapezoidWithAngleBottomBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        angle_bottom: 120.0 * PI / 180.0,
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        slope_bottom_radius: Length::new::<millimeter>(1.0),
        effective_opening_height: None,
        consider_tooth_tip_leakage: true,
    };
    let slot = OpenTrapezoidSlot::try_from(builder).unwrap();

    approx::assert_abs_diff_eq!(slot.outline().get::<meter>(), 0.0465666, epsilon = 1e-6);
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
        10.0,
        slot.magnetic_opening_height().get::<millimeter>(),
        epsilon = 1e-6
    );

    // Partial value calculation
    let contour = slot.contour();
    let bb = contour.bounding_box();
    approx::assert_abs_diff_eq!(
        7.625,
        slot.width(
            slot.height() - Length::new::<millimeter>(5.0),
            &contour,
            &bb
        )
        .get::<millimeter>(),
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        5.875,
        slot.width(
            slot.height() - Length::new::<millimeter>(15.0),
            &contour,
            &bb
        )
        .get::<millimeter>(),
        epsilon = 1e-3
    );

    // Check the slot leakage coefficients
    approx::assert_abs_diff_eq!(
        1.08016,
        slot.self_inductance_leakage_coefficient(0, &CoilLayout::Single),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(2.0, slot.leakage_coefficient_opening(), epsilon = 0.001);
    approx::assert_abs_diff_eq!(
        -0.05635,
        slot.leakage_coefficient_tooth_tip(Length::new::<millimeter>(1.0)),
        epsilon = 0.001
    );

    compare_to_reference(
        slot.drawables(CoilLayout::Single, true).as_slice(),
        "tests/img/open_trapezoid_slot_angle_bottom.png",
        None,
    );
}

#[test]
fn test_different_layers() {
    let builder = OpenTrapezoidWithAngleBottomBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_width: Length::new::<millimeter>(5.0),
        angle_bottom: 120.0 * PI / 180.0,
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        slope_bottom_radius: Length::new::<millimeter>(1.0),
        effective_opening_height: None,
        consider_tooth_tip_leakage: true,
    };
    let slot = OpenTrapezoidSlot::try_from(builder).unwrap();

    {
        // Horizontal
        let drawables = slot.drawables(CoilLayout::DoubleVertical, true);
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
    {
        // Vertical
        let drawables = slot.drawables(CoilLayout::DoubleHorizontal, true);
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
}

#[test]
fn test_open_slot_bottom_height() {
    let slot: OpenTrapezoidSlot = OpenTrapezoidWithBottomHeightBuilder {
        opening_width: Length::new::<millimeter>(5.0),
        height: Length::new::<millimeter>(5.0),
        opening_height: Length::new::<millimeter>(2.0),
        bottom_height: Length::new::<millimeter>(1.154),
        bottom_width: Length::new::<millimeter>(5.0),
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        slope_bottom_radius: Length::new::<millimeter>(1.0),
        effective_opening_height: None,
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
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_vert.png",
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
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        slope_bottom_radius: Length::new::<millimeter>(1.0),
        effective_opening_height: None,
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
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_hori.png",
        None,
    );
}

#[test]
fn test_open_slot_side_height_bugfix() {
    let bottom_radius = Length::new::<millimeter>(2.0);
    let angle_slot = PI / 18.0;
    let bottom_width = Length::new::<millimeter>(9.21);
    let slot: OpenTrapezoidSlot = OpenTrapezoidBuilder {
        bottom_width,
        opening_width: bottom_width
            - Length::new::<millimeter>(2.0 * 17.75) * (angle_slot / 2.0).sin(),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        effective_opening_height: None,
        angle_slot,
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_bugfix.png",
        None,
    );
}

#[test]
fn test_magnetic_slot_opening_value_invalid() {
    // Negative value for the magnetic slot opening height
    let angle = 30.0 / 180.0 * PI;
    let height = Length::new::<millimeter>(20.0);
    let opening_width = Length::new::<millimeter>(9.6);

    let slot = OpenTrapezoidSlot::new(
        opening_width + 2.0 * height * (0.5 * angle).sin(),
        opening_width,
        height,
        Length::new::<millimeter>(19.0),
        Length::new::<millimeter>(1.0),
        Some(Length::new::<millimeter>(-1.0)), // <== NEGATIVE VALUE!
        angle,
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    );
    assert!(slot.is_err());

    // Magnetic slot opening height larger than the total slot height
    let slot = OpenTrapezoidSlot::new(
        opening_width + 2.0 * height * (0.5 * angle).sin(),
        opening_width,
        height,
        Length::new::<millimeter>(19.0),
        Length::new::<millimeter>(1.0),
        Some(Length::new::<millimeter>(30.0)), // <== VALUE TOO LARGE
        angle,
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    );
    assert!(slot.is_err());
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
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        effective_opening_height: Some(Length::new::<millimeter>(1.0)),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, true);
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
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        slope_bottom_radius: Length::new::<millimeter>(1.0),
        effective_opening_height: None,
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let drawables = slot.drawables(CoilLayout::MultiVertical(1), true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_1_vertical.png",
        None,
    );

    let drawables = slot.drawables(CoilLayout::MultiVertical(2), true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_2_vertical.png",
        None,
    );

    let drawables = slot.drawables(CoilLayout::MultiVertical(3), true);
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

    let drawables = slot.drawables(CoilLayout::MultiVertical(4), true);
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/open_trapezoid_slot_4_vertical.png",
        None,
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
                angle_slot: 30.0 deg
                bottom_radius: 1 mm
                effective_opening_height: 1 mm
                consider_tooth_tip_leakage: false
                "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(CoilLayout::DoubleHorizontal, false);
        compare_to_reference(drawables.as_slice(), "tests/img/slot_MEAS.png", None);
    }
    {
        let yaml = indoc! {"
                        ---
                        opening_width: 9.6 mm
                        height: 20 mm
                        opening_height: 1.0 mm
                        angle_slot: 30.0 deg
                        bottom_radius: 1 mm
                        effective_opening_height: 1 mm
                        consider_tooth_tip_leakage: false
                        "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(CoilLayout::DoubleHorizontal, false);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/slot_trapezoid_open_opening_height_1mm.png",
            None,
        );
    }
    {
        let yaml = indoc! {"
                        ---
                        opening_width: 9.6 mm
                        height: 20 mm
                        opening_height: 5.0 mm
                        angle_slot: 30.0 deg
                        bottom_radius: 1 mm
                        effective_opening_height: 5 mm
                        consider_tooth_tip_leakage: false
                        "};
        let slot: OpenTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

        let drawables = slot.drawables(CoilLayout::DoubleHorizontal, false);
        compare_to_reference(
            drawables.as_slice(),
            "tests/img/slot_trapezoid_open_opening_height_5mm.png",
            None,
        );
    }
}
