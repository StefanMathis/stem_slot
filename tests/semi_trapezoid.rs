use cairo_viewport::*;
use indoc::indoc;
use planar_geo::prelude::*;
use std::f64::consts::{PI, TAU};
use stem_slot::{prelude::*, semi_trapezoid::*};

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
fn test_deserialize_bottom_with_width_and_height() {
    let data = indoc! {"
        ---
        bottom_width: 1.0 m
        side_bottom_width: 3.0 m
        bottom_height: 1.0 m
        slot_angle: 10.0 deg
        "};
    let bottom_angle: BottomAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(
        bottom_angle.value(),
        0.75 * PI - 0.5 * TAU / 36.0,
        epsilon = 1e-15
    );

    let data = indoc! {"
        ---
        10.0 deg
        "};
    let bottom_angle: BottomAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(bottom_angle.value(), TAU / 36.0, epsilon = 1e-15);

    let data = indoc! {"
        ---
        1.0
        "};
    let bottom_angle: BottomAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(bottom_angle.value(), 1.0, epsilon = 1e-15);
}

#[test]
fn test_deserialize_top_with_width_and_height() {
    let data = indoc! {"
        ---
        top_width: 1.0
        side_top_width: 3.0
        top_height: 1.0
        slot_angle: 10.0 deg
        "};
    let angle_top: TopAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    let slot_angle = TAU / 36.0; // 10°
    approx::assert_abs_diff_eq!(
        angle_top.value(),
        0.75 * PI + 0.5 * slot_angle,
        epsilon = 1e-15
    );

    let data = indoc! {"
        ---
        10.0 deg
        "};
    let angle_top: TopAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(angle_top.value(), TAU / 36.0, epsilon = 1e-15);

    let data = indoc! {"
        ---
        1.0
        "};
    let angle_top: TopAngleFromWidthHeight = serde_yaml::from_str(data).unwrap();
    approx::assert_abs_diff_eq!(angle_top.value(), 1.0, epsilon = 1e-15);
}

#[test]
fn test_angle_bottom_from_width_height() {
    let slot_angle = TAU / 36.0; // 10°

    // Case: No slope (bottom_width = side_bottom_width)
    approx::assert_abs_diff_eq!(
        PI - 0.5 * slot_angle,
        BottomAngleFromWidthHeight::Calculate {
            bottom_width: Length::new::<millimeter>(1.0),
            side_bottom_width: Length::new::<millimeter>(1.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: Almost no slope
    approx::assert_abs_diff_eq!(
        PI - 0.5 * slot_angle,
        BottomAngleFromWidthHeight::Calculate {
            bottom_width: Length::new::<millimeter>(1.0),
            side_bottom_width: Length::new::<millimeter>(1.0),
            bottom_height: Length::new::<millimeter>(0.01),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        1.9471774,
        BottomAngleFromWidthHeight::Calculate {
            bottom_width: Length::new::<millimeter>(1.0),
            side_bottom_width: Length::new::<millimeter>(3.0),
            bottom_height: Length::new::<millimeter>(0.5),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 45°
    approx::assert_abs_diff_eq!(
        0.75 * PI - 0.5 * slot_angle,
        BottomAngleFromWidthHeight::Calculate {
            bottom_width: Length::new::<millimeter>(1.0),
            side_bottom_width: Length::new::<millimeter>(3.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        2.59067858,
        BottomAngleFromWidthHeight::Calculate {
            bottom_width: Length::new::<millimeter>(1.0),
            side_bottom_width: Length::new::<millimeter>(2.0),
            bottom_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );
}

#[test]
fn test_angle_top_from_width_height() {
    let slot_angle = TAU / 36.0; // 10°

    // Case: No slope (bottom_width = side_bottom_width)
    approx::assert_abs_diff_eq!(
        PI + 0.5 * slot_angle,
        TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.0),
            side_top_width: Length::new::<millimeter>(1.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: Almost no slope
    approx::assert_abs_diff_eq!(
        PI + 0.5 * slot_angle,
        TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.0),
            side_top_width: Length::new::<millimeter>(1.0),
            top_height: Length::new::<millimeter>(0.01),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        1.94717747 + slot_angle,
        TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.0),
            side_top_width: Length::new::<millimeter>(3.0),
            top_height: Length::new::<millimeter>(0.5),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 45°
    approx::assert_abs_diff_eq!(
        0.75 * PI + 0.5 * slot_angle,
        TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.0),
            side_top_width: Length::new::<millimeter>(3.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );

    // Case: slope with 60°
    approx::assert_abs_diff_eq!(
        2.5906785 + slot_angle,
        TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.0),
            side_top_width: Length::new::<millimeter>(2.0),
            top_height: Length::new::<millimeter>(1.0),
            slot_angle
        }
        .value(),
        epsilon = 1e-6
    );
}

#[test]
fn test_properties() {
    let area = 155.65367; // mm²
    let outline = 0.0509879; // m
    {
        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width: Length::new::<millimeter>(10.0),
            opening_width: Length::new::<millimeter>(2.0),
            height: Length::new::<millimeter>(20.0),
            opening_height: Length::new::<millimeter>(2.0),
            slot_angle: 10.0 * PI / 180.0,
            bottom_radius: Length::new::<millimeter>(0.0),
            top_radius: Length::new::<millimeter>(0.0),
            opening_radius: Length::new::<millimeter>(0.0),
            consider_tooth_tip_leakage: true,
        }
        .try_into()
        .unwrap();

        approx::assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), area, epsilon = 1e-5);

        assert!(slot.is_open());
        approx::assert_abs_diff_eq!(
            slot.outline().length(),
            outline + 2.0 * slot.opening_height().get::<meter>(),
            epsilon = 1e-6
        );

        // Sum up the partial slot outlines
        approx::assert_abs_diff_eq!(
            slot.layer_outlines(0, &CoilLayout::Single)
                .length()
                .get::<meter>(),
            outline,
            epsilon = 1e-6
        );

        let pt1 = slot
            .layer_outlines(0, &CoilLayout::DoubleHorizontal)
            .length()
            .get::<meter>();
        let pt2 = slot
            .layer_outlines(1, &CoilLayout::DoubleHorizontal)
            .length()
            .get::<meter>();
        approx::assert_abs_diff_eq!(pt1, pt2, epsilon = 1e-6); // Both outlines cover one half of the slot
        approx::assert_abs_diff_eq!(pt1 + pt2, outline, epsilon = 1e-6);

        let pt1 = slot
            .layer_outlines(0, &CoilLayout::DoubleVertical)
            .length()
            .get::<meter>();
        let pt2 = slot
            .layer_outlines(1, &CoilLayout::DoubleVertical)
            .length()
            .get::<meter>();
        assert!(pt1 > pt2); // pt1 is much larger since it includes the slot bottom
        approx::assert_abs_diff_eq!(pt1 + pt2, outline, epsilon = 1e-6);

        let pt1 = slot
            .layer_outlines(0, &CoilLayout::MultiVertical(2))
            .length()
            .get::<meter>();
        let pt2 = slot
            .layer_outlines(1, &CoilLayout::MultiVertical(2))
            .length()
            .get::<meter>();
        assert!(pt1 > pt2); // pt1 is much larger since it includes the slot bottom
        approx::assert_abs_diff_eq!(pt1 + pt2, outline, epsilon = 1e-6);

        let pt1 = slot
            .layer_outlines(0, &CoilLayout::Quadruple)
            .length()
            .get::<meter>();
        let pt2 = slot
            .layer_outlines(1, &CoilLayout::Quadruple)
            .length()
            .get::<meter>();
        let pt3 = slot
            .layer_outlines(2, &CoilLayout::Quadruple)
            .length()
            .get::<meter>();
        let pt4 = slot
            .layer_outlines(3, &CoilLayout::Quadruple)
            .length()
            .get::<meter>();
        approx::assert_abs_diff_eq!(pt1 + pt2 + pt3 + pt4, outline, epsilon = 1e-6);
    }

    {
        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width: Length::new::<millimeter>(10.0),
            opening_width: Length::new::<millimeter>(0.0),
            height: Length::new::<millimeter>(20.0),
            opening_height: Length::new::<millimeter>(2.0),
            slot_angle: 10.0 * PI / 180.0,
            bottom_radius: Length::new::<millimeter>(0.0),
            top_radius: Length::new::<millimeter>(0.0),
            opening_radius: Length::new::<millimeter>(0.0),
            consider_tooth_tip_leakage: true,
        }
        .try_into()
        .unwrap();

        assert!(!slot.is_open());

        approx::assert_abs_diff_eq!(
            slot.area().get::<square_millimeter>(),
            area - 4.0,
            epsilon = 1e-5
        );

        approx::assert_abs_diff_eq!(slot.outline().length(), outline + 0.002, epsilon = 1e-6);
    }
}

#[test]
fn test_current_displacement_coefficients() {
    let frequency = Frequency::new::<hertz>(100.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m
    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    // Slot from [Mat19]
    let bottom_radius = Length::new::<millimeter>(0.5);
    let slot_angle = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (slot_angle / 2.0).sin());

    let slot: SemiTrapezoidSlot = SemiTrapezoidBuilder {
        bottom_width,
        top_width: bottom_width - 2.0 * Length::new::<millimeter>(17.0) * (slot_angle / 2.0).sin(),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle,
        bottom_angle: BottomAngleFromWidthHeight::new_no_slope(slot_angle),
        angle_top: TopAngleFromWidthHeight::new_no_slope(slot_angle),
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let coeffs = slot.current_displacement_coefficients(50).eval(
        frequency,
        el_conductivity,
        rel_permeability,
    );

    // kr
    approx::assert_abs_diff_eq!(coeffs.resistance, 2.23397, epsilon = 1e-6);

    // kx
    approx::assert_abs_diff_eq!(coeffs.inductance, 0.810344, epsilon = 1e-6);
}

#[test]
fn test_slices() {
    let bottom_radius = Length::new::<millimeter>(0.5);
    let slot_angle = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (slot_angle / 2.0).sin());

    let slot: SemiTrapezoidSlot = SemiTrapezoidBuilder {
        bottom_width,
        top_width: bottom_width - 2.0 * Length::new::<millimeter>(17.0) * (slot_angle / 2.0).sin(),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle,
        bottom_angle: BottomAngleFromWidthHeight::new_no_slope(slot_angle),
        angle_top: TopAngleFromWidthHeight::new_no_slope(slot_angle),
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let bbs = slot.slices(50);
    assert_eq!(bbs.len(), 66);
}

#[test]
fn test_semi_trapezoid_side_height() {
    let bottom_radius = Length::new::<millimeter>(0.5);
    let slot_angle = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (slot_angle / 2.0).sin());

    let slot = SemiTrapezoidSlot::new(
        bottom_width,
        bottom_width - Length::new::<millimeter>(2.0 * 17.0) * (slot_angle / 2.0).sin(),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(17.0),
        Length::new::<millimeter>(0.75),
        slot_angle,
        BottomAngleFromWidthHeight::new_no_slope(slot_angle).value(),
        TopAngleFromWidthHeight::new_no_slope(slot_angle).value(),
        bottom_radius,
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();

    approx::assert_abs_diff_eq!(
        132.38,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );
    approx::assert_abs_diff_eq!(
        132.38 + 1.5,
        slot.area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    approx::assert_abs_diff_eq!(
        slot.bottom_width().get::<millimeter>(),
        slot.bottom_side_width().get::<millimeter>(),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        6.33381,
        slot.top_width().get::<millimeter>(),
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        9.297,
        slot.bottom_width().get::<millimeter>(),
        epsilon = 1e-3
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, true).as_slice(),
        "tests/img/semi_trapezoid_single_layer.png",
        None,
    );
}

#[test]
fn test_leakage_coefficients() {
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<millimeter>(10.0),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        top_radius: Length::new::<millimeter>(1.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Winding area
    let winding_area_leakage_coeff = 0.86432;
    let coil_layout = CoilLayout::Single;
    approx::assert_abs_diff_eq!(
        winding_area_leakage_coeff,
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        epsilon = 0.001
    );

    let coil_layout = CoilLayout::DoubleHorizontal;
    approx::assert_abs_diff_eq!(
        winding_area_leakage_coeff,
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(
        winding_area_leakage_coeff,
        slot.self_inductance_leakage_coefficient(1, &coil_layout),
        epsilon = 0.001
    );

    let coil_layout = CoilLayout::DoubleVertical;
    approx::assert_abs_diff_eq!(
        1.57181,
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(
        0.47081,
        slot.self_inductance_leakage_coefficient(1, &coil_layout),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(
        2.04262,
        slot.self_inductance_leakage_coefficient(0, &coil_layout)
            + slot.self_inductance_leakage_coefficient(1, &coil_layout),
        epsilon = 0.001
    );

    approx::assert_abs_diff_eq!(
        0.6749,
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        epsilon = 0.001
    );
    approx::assert_abs_diff_eq!(
        0.6749,
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        epsilon = 0.001
    );
}

#[test]
fn test_plot_with_and_without_slot_opening() {
    let slot_angle = PI / 3.0;
    let height = Length::new::<millimeter>(7.0);
    let opening_height = Length::new::<millimeter>(1.0);
    let bottom_width = Length::new::<millimeter>(15.3);

    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width,
        opening_width: Length::new::<millimeter>(2.0),
        height,
        opening_height,
        slot_angle,
        bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    let mut drawables = slot.drawables(&CoilLayout::Single, true);

    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width,
        opening_width: Length::new::<millimeter>(0.0),
        height,
        opening_height,
        slot_angle,
        bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    drawables.extend(slot.drawables(&CoilLayout::Single, true));

    compare_to_reference(
        drawables.as_slice(),
        "tests/img/semi_trapezoid_w_and_wo_opening.png",
        None,
    );
}

#[test]
fn test_tooth_width_deserialize() {
    // Read from the database
    let yaml = indoc! {"
              ---
              tooth_width: 9.2 mm
              air_gap_radius: 38.2 mm
              yoke_radius: 63 mm
              slots: 12
              opening_width: 4 mm
              height: 19.35 mm
              opening_height: 1 mm
              bottom_radius: 2.0 mm
              top_radius: 2.0 mm
              opening_radius: 1.0 mm
              consider_tooth_tip_leakage: false
              "};

    let slot: SemiTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();
    approx::assert_abs_diff_eq!(
        296.42,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleHorizontal, true)
            .as_slice(),
        "tests/img/semi_trapezoid_parallel_teeth_dl.png",
        None,
    );
}

#[test]
fn test_semi_trapezoid_no_slopes_deserialize() {
    // Read from the database
    let yaml = indoc! {"
                ---
                bottom_width: 0.01
                opening_width: 0.002
                height: 0.02
                opening_height: 0.002
                slot_angle: 10deg
                bottom_radius: 0.002
                top_radius: 0.001
                opening_radius: 0.0
                consider_tooth_tip_leakage: true
                "};

    let slot: SemiTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();
    approx::assert_abs_diff_eq!(
        149.21,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleVertical, true).as_slice(),
        "tests/img/semi_trapezoid_hori_dl.png",
        None,
    );
}

#[test]
fn test_semi_trapezoid_side_top_width_deserialize() {
    // Read from the database
    let yaml = indoc! {"
                ---
                bottom_width: 6.76 mm
                top_width: 1.5 mm
                side_top_width: 8 mm
                opening_width: 1.5 mm
                height: 6.79 mm
                opening_height: 0.75 mm
                slot_angle: -360/28 deg
                bottom_angle:
                  bottom_width: 6.76 mm
                  side_bottom_width: 6.76 mm
                  bottom_height: 0.0 mm
                  slot_angle: -360/28 deg
                angle_top:
                  top_width: 1.5 mm
                  side_top_width: 8 mm
                  top_height: 0.5 mm
                  slot_angle: -360/28 deg
                bottom_radius: 0.0 mm
                slope_bottom_radius: 0.0 mm
                top_radius: 0.0 mm
                slope_top_radius: 0.0 mm
                opening_radius: 0.0 mm
                consider_tooth_tip_leakage: true
                "};

    let slot: SemiTrapezoidSlot = serde_yaml::from_str(yaml).unwrap();

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, true).as_slice(),
        "tests/img/semi_trapezoid_inner.png",
        None,
    );
}

#[test]
fn test_contour_main_body() {
    // Values from the from_winding method of CoreRotSlotted

    let yoke_radius = 20e-3;
    let b_opening: f64 = 2e-3;
    let h_opening = 1e-3;

    // Ratio between the angle covered by a tooth and by a slot bottom
    let ratio_tooth_slot_bottom = 3.0;

    // Angle covered by one tooth
    let slots = 6;
    let slot_angle = TAU / slots as f64;
    let alpha = slot_angle * 1.0 / (1.0 + ratio_tooth_slot_bottom);
    let beta = slot_angle - alpha;

    // Slot bottom width
    let b_bottom = 2.0 * yoke_radius * (beta / 2.0).sin();
    let h_yoke = yoke_radius * (1.0 - (beta / 2.0).cos());

    // Scale the air gap radius by the number of slots up to 0.9.
    // The scaling formula was created "by hand" to give a good visual
    // representation, the values are chosen arbitrarily and have no deeper meaning.
    let scale_air_gap = 0.1 + 0.8 * (1.0 - 1.0 / (slots as f64).sqrt());
    let air_gap_radius = yoke_radius * scale_air_gap;

    // Calculate the slot height
    let h = yoke_radius - h_yoke - 0.5 * (4.0 * air_gap_radius.powi(2) - b_opening.powi(2)).sqrt();

    // Create slot and core object (they are just used for plotting purposes)
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<meter>(b_bottom),
        opening_width: Length::new::<meter>(b_opening),
        height: Length::new::<meter>(h),
        opening_height: Length::new::<meter>(h_opening),
        slot_angle,
        bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    {
        let contour = Contour::from(slot.outline().into_owned());
        approx::assert_abs_diff_eq!(contour.area(), 7.350393e-5, epsilon = 1e-9);
    }
    {
        let contour = Contour::from(slot.outline_winding_area());
        approx::assert_abs_diff_eq!(contour.area(), 7.150393e-5, epsilon = 1e-9);
    }
}

#[test]
fn test_inner_slot() {
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithSideTopWidthBuilder {
        bottom_width: Length::new::<millimeter>(6.76),
        top_width: Length::new::<millimeter>(1.5),
        opening_width: Length::new::<millimeter>(1.5),
        height: Length::new::<millimeter>(6.79),
        side_top_width: Length::new::<millimeter>(8.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle: PI / 14.0,
        bottom_angle: BottomAngleFromWidthHeight::new_no_slope(PI / 14.0),
        angle_top: TopAngleFromWidthHeight::Calculate {
            top_width: Length::new::<millimeter>(1.5),
            side_top_width: Length::new::<millimeter>(8.0),
            top_height: Length::new::<millimeter>(0.5),
            slot_angle: PI / 14.0,
        },
        bottom_radius: Length::new::<millimeter>(0.0),
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, true).as_slice(),
        "tests/img/semi_trapezoid_inner.png",
        None,
    );
}

#[test]
fn test_from_rotary_core() {
    let slot: SemiTrapezoidSlot = SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder {
        tooth_width: Length::new::<millimeter>(3.415),
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        slots: 36,
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        opening_height: Length::new::<millimeter>(0.75),
        bottom_radius: Length::new::<millimeter>(0.5),
        top_radius: Length::new::<millimeter>(1.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, true).as_slice(),
        "tests/img/semi_trapezoid_from_rotative_core_outer.png",
        None,
    );
}

#[test]
fn test_semi_trapezoid_inner_stator() {
    let bottom_radius = Length::new::<millimeter>(0.5);
    let slot_angle = -PI / 18.0;
    let bottom_width = Length::new::<millimeter>(6.33381);
    let top_width = Length::new::<millimeter>(9.297);

    let slot: SemiTrapezoidSlot = SemiTrapezoidBuilder {
        bottom_width,
        top_width,
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle,
        bottom_angle: BottomAngleFromWidthHeight::new_no_slope(slot_angle),
        angle_top: TopAngleFromWidthHeight::new_no_slope(slot_angle),
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    approx::assert_abs_diff_eq!(
        132.25,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );
    approx::assert_abs_diff_eq!(
        132.25 + 1.5,
        slot.area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    approx::assert_abs_diff_eq!(
        slot.bottom_width().get::<millimeter>(),
        slot.bottom_side_width().get::<millimeter>(),
        epsilon = 1e-6
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleVertical, true).as_slice(),
        "tests/img/semi_trapezoid_inner_stator_double_layer_hori.png",
        None,
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleHorizontal, true)
            .as_slice(),
        "tests/img/semi_trapezoid_inner_stator_double_layer_vert.png",
        None,
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, true).as_slice(),
        "tests/img/semi_trapezoid_inner_stator.png",
        None,
    );

    // Image comparison: Slices
    let bbs = slot.slices(10);
    assert_eq!(bbs.len(), 29);

    let mut style = Style::default();
    style.background_color = stem_slot::ORANGE;

    let drawables: Vec<DrawableCow> = bbs
        .into_iter()
        .map(|bb| DrawableCow::new(bb, style.clone()))
        .collect();

    compare_to_reference(
        drawables.as_slice(),
        "tests/img/semi_trapezoid_inner_stator_slices.png",
        None,
    );
}

#[test]
fn test_semi_trapezoid_creation_no_slopes() {
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<millimeter>(10.0),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        slot_angle: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        top_radius: Length::new::<millimeter>(1.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Check some geometric parameters
    approx::assert_abs_diff_eq!(
        149.21,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );
    approx::assert_abs_diff_eq!(
        149.21 + 4.0,
        slot.area().get::<square_millimeter>(),
        epsilon = 1e-2
    );
    approx::assert_abs_diff_eq!(85.0 / 180.0 * PI, slot.bottom_angle(), epsilon = 1e-5);
    approx::assert_abs_diff_eq!(95.0 / 180.0 * PI, slot.angle_top(), epsilon = 1e-5);

    approx::assert_abs_diff_eq!(
        slot.bottom_width().get::<millimeter>(),
        slot.bottom_side_width().get::<millimeter>(),
        epsilon = 1e-6
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleVertical, true).as_slice(),
        "tests/img/semi_trapezoid_hori_dl.png",
        None,
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleHorizontal, true)
            .as_slice(),
        "tests/img/semi_trapezoid_vert_dl.png",
        None,
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::DoubleHorizontal, false)
            .as_slice(),
        "tests/img/semi_trapezoid_vert_no_opening_dl.png",
        None,
    );

    compare_to_reference(
        slot.drawables(&CoilLayout::Single, false).as_slice(),
        "tests/img/semi_trapezoid_vert_no_opening_sl.png",
        None,
    );

    let drawables = slot.drawables(&CoilLayout::Quadruple, true);
    let view = Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap();
    compare_to_reference(
        drawables.as_slice(),
        "tests/img/semi_trapezoid_vert_no_opening_ql.png",
        Some(view.clone()),
    );

    compare_to_reference(
        &[drawables[0].clone()],
        "tests/img/semi_trapezoid_vert_no_opening_ql_layer_1.png",
        Some(view.clone()),
    );

    compare_to_reference(
        &[drawables[1].clone()],
        "tests/img/semi_trapezoid_vert_no_opening_ql_layer_2.png",
        Some(view.clone()),
    );

    compare_to_reference(
        &[drawables[2].clone()],
        "tests/img/semi_trapezoid_vert_no_opening_ql_layer_3.png",
        Some(view.clone()),
    );

    compare_to_reference(
        &[drawables[3].clone()],
        "tests/img/semi_trapezoid_vert_no_opening_ql_layer_4.png",
        Some(view.clone()),
    );
}
