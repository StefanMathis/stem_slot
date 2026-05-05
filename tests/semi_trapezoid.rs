#[test]
fn test_slot_trapezoid_semi() {
    {
        let slot = SlotTrapezoidSemi::new_without_slopes(
            Length::new::<millimeter>(10.0),
            Length::new::<millimeter>(2.0),
            Length::new::<millimeter>(20.0),
            Length::new::<millimeter>(2.0),
            10.0 * PI / 180.0,
            Length::new::<millimeter>(2.0),
            Length::new::<millimeter>(1.0),
            Length::new::<millimeter>(0.0),
            true,
        )
        .unwrap();

        let outline = 0.0481914; // m

        assert!(slot.is_open());
        approx::assert_abs_diff_eq!(
            slot.outline().get::<meter>(),
            outline + 2.0 * slot.opening_height().get::<meter>(),
            epsilon = 1e-6
        );

        // Sum up the partial slot outlines
        approx::assert_abs_diff_eq!(
            slot.layer_outline(0, &CoilLayout::Single).get::<meter>(),
            outline,
            epsilon = 1e-6
        );

        let pt1 = slot.layer_outline(0, &CoilLayout::DoubleHorizontal);
        let pt2 = slot.layer_outline(1, &CoilLayout::DoubleHorizontal);
        approx::assert_abs_diff_eq!(pt1.get::<meter>(), pt2.get::<meter>(), epsilon = 1e-6); // Both outlines cover one half of the slot
        approx::assert_abs_diff_eq!((pt1 + pt2).get::<meter>(), outline, epsilon = 1e-6);

        let pt1 = slot.layer_outline(0, &CoilLayout::DoubleVertical);
        let pt2 = slot.layer_outline(1, &CoilLayout::DoubleVertical);
        assert!(pt1 > pt2); // pt1 is much larger since it includes the slot bottom
        approx::assert_abs_diff_eq!((pt1 + pt2).get::<meter>(), outline, epsilon = 1e-6);

        let pt1 = slot.layer_outline(0, &CoilLayout::MultiVertical(2));
        let pt2 = slot.layer_outline(1, &CoilLayout::MultiVertical(2));
        assert!(pt1 > pt2); // pt1 is much larger since it includes the slot bottom
        approx::assert_abs_diff_eq!((pt1 + pt2).get::<meter>(), outline, epsilon = 1e-6);

        let pt1 = slot.layer_outline(0, &CoilLayout::Quadruple);
        let pt2 = slot.layer_outline(1, &CoilLayout::Quadruple);
        let pt3 = slot.layer_outline(2, &CoilLayout::Quadruple);
        let pt4 = slot.layer_outline(3, &CoilLayout::Quadruple);
        approx::assert_abs_diff_eq!(
            (pt1 + pt2 + pt3 + pt4).get::<meter>(),
            outline,
            epsilon = 1e-6
        );
    }

    {
        let slot = SlotTrapezoidSemi::new_without_slopes(
            Length::new::<millimeter>(10.0),
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(20.0),
            Length::new::<millimeter>(2.0),
            10.0 * PI / 180.0,
            Length::new::<millimeter>(2.0),
            Length::new::<millimeter>(1.0),
            Length::new::<millimeter>(0.0),
            true,
        )
        .unwrap();

        assert!(!slot.is_open());
        approx::assert_abs_diff_eq!(slot.outline().get::<meter>(), 0.0501914, epsilon = 1e-6);
    }
}

#[test]
fn test_plot_without_slot_opening() {
    let angle_slot = PI / 3.0;
    let h = Length::new::<millimeter>(7.0);
    let opening_width = Length::new::<millimeter>(2.0);
    let opening_height = Length::new::<millimeter>(1.0);
    let b_bottom = Length::new::<millimeter>(15.3);
    let slot = SemiTrapezoidSlot::new_without_slopes(
        b_bottom,
        opening_width,
        h,
        opening_height,
        angle_slot,
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        false,
    )
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, false);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_wo_opening_in_plot.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
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

    let slot: SemiTrapezoidSlot = create_dbm().from_str(yaml).unwrap();
    approx::assert_abs_diff_eq!(
        296.42,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    // Image comparison
    let drawables = slot.drawables(CoilLayout::DoubleHorizontal, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_parallel_teeth_dl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_semi_trapezoid_slot_no_slopes_deserialize() {
    // Read from the database
    let yaml = indoc! {"
                ---
                bottom_width: 0.01
                opening_width: 0.002
                height: 0.02
                opening_height: 0.002
                angle_slot: 10deg
                bottom_radius: 0.002
                top_radius: 0.001
                opening_radius: 0.0
                consider_tooth_tip_leakage: true
                "};

    let slot: SemiTrapezoidSlot = create_dbm().from_str(yaml).unwrap();
    approx::assert_abs_diff_eq!(
        149.21,
        slot.winding_area().get::<square_millimeter>(),
        epsilon = 1e-2
    );

    // Image comparison
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_hori_dl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_semi_trapezoid_slot_side_top_width_deserialize() {
    // Read from the database
    let yaml = indoc! {"
                ---
                bottom_width: 6.76 mm
                top_width: 1.5 mm
                side_top_width: 8 mm
                opening_width: 1.5 mm
                height: 6.79 mm
                opening_height: 0.75 mm
                angle_slot: -360/28 deg
                angle_bottom:
                  bottom_width: 6.76 mm
                  side_bottom_width: 6.76 mm
                  bottom_height: 0.0 mm
                  angle_slot: -360/28 deg
                angle_top:
                  top_width: 1.5 mm
                  side_top_width: 8 mm
                  top_height: 0.5 mm
                  angle_slot: -360/28 deg
                bottom_radius: 0.0 mm
                slope_bottom_radius: 0.0 mm
                top_radius: 0.0 mm
                slope_top_radius: 0.0 mm
                opening_radius: 0.0 mm
                consider_tooth_tip_leakage: true
                "};

    let slot: SemiTrapezoidSlot = create_dbm().from_str(yaml).unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_inner.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_semi_trapezoid_slot_creation_no_slopes() {
    let slot = SemiTrapezoidSlot::new_without_slopes(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        10.0 * PI / 180.0,
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        true,
    )
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
    approx::assert_abs_diff_eq!(85.0 / 180.0 * PI, slot.angle_bottom, epsilon = 1e-5);
    approx::assert_abs_diff_eq!(95.0 / 180.0 * PI, slot.angle_top, epsilon = 1e-5);
    let dep_params = slot.dependent_parameters();
    approx::assert_abs_diff_eq!(
        slot.bottom_width.get::<millimeter>(),
        dep_params.side_bottom_width.get::<millimeter>(),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        0.0,
        dep_params._bottom_height.get::<millimeter>(),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        0.0,
        dep_params.top_height.get::<millimeter>(),
        epsilon = 1e-6
    );

    approx::assert_abs_diff_eq!(slot.mean_width().get::<millimeter>(), 8.425, epsilon = 1e-3);

    // Partial value calculation
    let contour: Contour = slot.shape().into();
    let bb = contour.bounding_box();
    approx::assert_abs_diff_eq!(
        9.125,
        slot.width(
            slot.height() - Length::new::<millimeter>(5.0),
            &contour,
            &bb
        )
        .get::<millimeter>(),
        epsilon = 1e-3
    );
    approx::assert_abs_diff_eq!(
        7.375,
        slot.width(
            slot.height() - Length::new::<millimeter>(15.0),
            &contour,
            &bb
        )
        .get::<millimeter>(),
        epsilon = 1e-3
    );

    // Image comparison
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_hori_dl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let drawables = slot.drawables(CoilLayout::DoubleHorizontal, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_dl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let drawables = slot.drawables(CoilLayout::DoubleHorizontal, false);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_dl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let drawables = slot.drawables(CoilLayout::Single, false);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_sl.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Shapes of a four-layer winding slots
    let drawables = slot.drawables(CoilLayout::Quadruple, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_ql.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Plot individual components
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_ql_layer_1.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            drawables[0].draw(cr);
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_ql_layer_2.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            drawables[1].draw(cr);
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_ql_layer_3.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            drawables[2].draw(cr);
        });
    };

    assert!(compare_or_create(path, &callback).is_ok());
    let path = std::path::Path::new("img/slot_trapezoid_semi_vert_no_opening_ql_layer_4.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            drawables[3].draw(cr);
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_leakage_coefficients() {
    let slot = SemiTrapezoidSlot::new_without_slopes(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        10.0 * PI / 180.0,
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        true,
    )
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
fn test_semi_trapezoid_slot_side_height() {
    // Slot from [Mat19]
    let bottom_radius = Length::new::<millimeter>(0.5);
    let angle_slot = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (angle_slot / 2.0).sin());

    let slot = SemiTrapezoidSlot::new(
        bottom_width,
        bottom_width - Length::new::<millimeter>(2.0 * 17.0) * (angle_slot / 2.0).sin(),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(17.0),
        Length::new::<millimeter>(0.75),
        angle_slot,
        angle_bottom_no_slope(angle_slot),
        angle_top_no_slope(angle_slot),
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

    let dep_params = slot.dependent_parameters();
    approx::assert_abs_diff_eq!(
        slot.bottom_width.get::<millimeter>(),
        dep_params.side_bottom_width.get::<millimeter>(),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(6.33381, slot.top_width.get::<millimeter>(), epsilon = 1e-3);
    approx::assert_abs_diff_eq!(9.297, slot.bottom_width.get::<millimeter>(), epsilon = 1e-3);

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_single_layer.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_semi_trapezoid_slot_inner_stator() {
    let bottom_radius = Length::new::<millimeter>(0.5);
    let angle_slot = -PI / 18.0;
    let bottom_width = Length::new::<millimeter>(6.33381);
    let top_width = Length::new::<millimeter>(9.297);

    let slot: SemiTrapezoidSlot = NewWithSideHeight {
        bottom_width,
        top_width,
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        angle_slot,
        angle_bottom: angle_bottom_no_slope(angle_slot),
        angle_top: angle_top_no_slope(angle_slot),
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

    let dep_params = slot.dependent_parameters();
    approx::assert_abs_diff_eq!(
        slot.bottom_width.get::<millimeter>(),
        dep_params.side_bottom_width.get::<millimeter>(),
        epsilon = 1e-6
    );

    // Image comparison: Normal slots
    let drawables = slot.drawables(CoilLayout::DoubleVertical, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_inner_stator_double_layer_hori.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let drawables = slot.drawables(CoilLayout::DoubleHorizontal, true);
    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_inner_stator_double_layer_vert.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    let drawables = slot.drawables(CoilLayout::Single, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_inner_stator.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Image comparison: Slices
    let shapes = slot.slice_shapes(10);
    assert_eq!(shapes.len(), 28);
    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_inner_stator_slices.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_slices() {
    // Slot from [Mat19]
    let bottom_radius = Length::new::<millimeter>(0.5);
    let angle_slot = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (angle_slot / 2.0).sin());

    let slot: SemiTrapezoidSlot = NewWithSideHeight {
        bottom_width,
        top_width: bottom_width - 2.0 * Length::new::<millimeter>(17.0) * (angle_slot / 2.0).sin(),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        angle_slot,
        angle_bottom: angle_bottom_no_slope(angle_slot),
        angle_top: angle_top_no_slope(angle_slot),
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let shapes = slot.shapes(CoilLayout::Single, true);

    let [x_ul, y_ul, x_lr, y_lr] = slot.slices(50, &shapes[0].contour());

    // Check that every slice is not degenerated (i.e. has a surface area greater
    // than zero)
    for ii in 0..x_ul.len() {
        assert!(x_lr[ii] - x_ul[ii] > 0.0);
        assert!(y_ul[ii] - y_lr[ii] > 0.0);
    }
}

#[test]
fn test_current_displacement_coefficients() {
    let frequency = Frequency::new::<hertz>(100.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m
    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    // Slot from [Mat19]
    let bottom_radius = Length::new::<millimeter>(0.5);
    let angle_slot = PI / 18.0;
    let bottom_width =
        Length::new::<millimeter>(8.21) + 2.0 * bottom_radius * (1.0 + (angle_slot / 2.0).sin());

    let slot: SemiTrapezoidSlot = NewWithSideHeight {
        bottom_width,
        top_width: bottom_width - 2.0 * Length::new::<millimeter>(17.0) * (angle_slot / 2.0).sin(),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        angle_slot,
        angle_bottom: angle_bottom_no_slope(angle_slot),
        angle_top: angle_top_no_slope(angle_slot),
        bottom_radius,
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Use the numeric approach
    let coeffs =
        slot.current_displacement_coefficients(frequency, el_conductivity, rel_permeability);
    approx::assert_abs_diff_eq!(coeffs.resistance_coefficient, 1.664930, epsilon = 1e-6); // kr
    approx::assert_abs_diff_eq!(coeffs.inductance_coefficient, 0.635196, epsilon = 1e-6);
    // kx
}

#[test]
fn test_from_rotative_core() {
    let tooth_width = Length::new::<millimeter>(3.415);
    let air_gap_radius = Length::new::<millimeter>(55.0);
    let yoke_radius = Length::new::<millimeter>(85.0);
    let height = Length::new::<millimeter>(17.75);

    let slot = SemiTrapezoidSlot::new_from_tooth_width_without_slopes_rot(
        tooth_width,
        air_gap_radius,
        yoke_radius,
        36,
        Length::new::<millimeter>(2.0),
        height,
        Length::new::<millimeter>(0.75),
        Length::new::<millimeter>(0.5),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_from_rotative_core_outer.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // ================================
    // Inner core

    let yoke_radius = Length::new::<millimeter>(30.0);
    let slot = SemiTrapezoidSlot::new_from_tooth_width_without_slopes_rot(
        tooth_width,
        air_gap_radius,
        yoke_radius,
        36,
        Length::new::<millimeter>(2.0),
        height,
        Length::new::<millimeter>(0.75),
        Length::new::<millimeter>(0.5),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_from_rotative_core_inner.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_inner_slot() {
    let slot: SemiTrapezoidSlot = NewWithSideTopWidth {
        bottom_width: Length::new::<millimeter>(6.76),
        top_width: Length::new::<millimeter>(1.5),
        opening_width: Length::new::<millimeter>(1.5),
        height: Length::new::<millimeter>(6.79),
        side_top_width: Length::new::<millimeter>(8.0),
        opening_height: Length::new::<millimeter>(0.75),
        angle_slot: PI / 14.0,
        angle_bottom: angle_bottom_no_slope(PI / 14.0),
        angle_top: angle_top_from_width_height(
            Length::new::<millimeter>(1.5),
            Length::new::<millimeter>(8.0),
            Length::new::<millimeter>(0.5),
            PI / 14.0,
        ),
        bottom_radius: Length::new::<millimeter>(0.0),
        slope_bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        slope_top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    // Image comparison
    let drawables = slot.drawables(CoilLayout::Single, true);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/slot_trapezoid_semi_inner.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr);
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_contour_main_body() {
    // Values from the from_winding method of CoreRotSlotted

    let yoke_radius = 20e-3;
    let b_opening: f64 = 2e-3;
    let h_opening = 1e-3;

    // Ratio between the angle covered by a tooth and by a slot bottom
    let ratio = 3.0;

    // Angle covered by one tooth
    let slots = 6;
    let angle_slot = TAU / slots as f64;
    let alpha = angle_slot * 1.0 / (1.0 + ratio);
    let beta = angle_slot - alpha;

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
    let slot = SemiTrapezoidSlot::new_without_slopes(
        Length::new::<meter>(b_bottom),
        Length::new::<meter>(b_opening),
        Length::new::<meter>(h),
        Length::new::<meter>(h_opening),
        angle_slot,
        Length::new::<meter>(0.0),
        Length::new::<meter>(0.0),
        Length::new::<meter>(0.0),
        false,
    )
    .expect("slot can always be created from the given value.");

    {
        let contour = slot.contour();
        approx::assert_abs_diff_eq!(contour.area(), 7.350393e-5, epsilon = 1e-9);
    }
    {
        let contour = slot.contour_main_body();
        approx::assert_abs_diff_eq!(contour.area(), 7.150393e-5, epsilon = 1e-9);
    }
}
