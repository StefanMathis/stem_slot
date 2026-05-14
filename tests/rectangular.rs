use std::f64::consts::TAU;

use cairo_viewport::*;
use indoc::indoc;
use planar_geo::prelude::*;
use stem_slot::{current_displacement::phase_velocity, prelude::*};

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
fn test_slot_outline() {
    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();
    approx::assert_abs_diff_eq!(
        slot.outline().length(),
        (2.0 * height + width).get::<meter>(),
        epsilon = 1e-6
    );

    let contour = Contour::from(slot.outline().into_owned());
    let vertices: Vec<[f64; 2]> = contour.points().collect();
    approx::assert_abs_diff_eq!(vertices[0][0], opening_width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[0][1], 0.0);
    approx::assert_abs_diff_eq!(vertices[1][0], opening_width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[1][1], opening_height.get::<meter>());
    approx::assert_abs_diff_eq!(vertices[2][0], width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[2][1], height.get::<meter>());
    approx::assert_abs_diff_eq!(vertices[3][0], -width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[3][1], height.get::<meter>());
    approx::assert_abs_diff_eq!(vertices[4][0], -width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[4][1], opening_height.get::<meter>());
    approx::assert_abs_diff_eq!(vertices[5][0], -opening_width.get::<meter>() / 2.0);
    approx::assert_abs_diff_eq!(vertices[5][1], 0.0);
}

#[test]
fn test_plot() {
    {
        let opening_height = Length::new::<millimeter>(1.0);
        let opening_width = Length::new::<millimeter>(3.0);
        let width = Length::new::<millimeter>(8.0);
        let height = Length::new::<millimeter>(20.0);
        let slot =
            RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

        compare_to_reference(
            slot.drawables(CoilLayout::Single, true).as_slice(),
            "tests/img/rectangular_w_opening.png",
            None,
        );

        compare_to_reference(
            slot.drawables(CoilLayout::Single, false).as_slice(),
            "tests/img/rectangular_wo_opening.png",
            None,
        );
    }
    {
        let opening_height = Length::new::<millimeter>(1.0);
        let opening_width = Length::new::<millimeter>(0.0);
        let width = Length::new::<millimeter>(8.0);
        let height = Length::new::<millimeter>(20.0);
        let slot =
            RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

        compare_to_reference(
            slot.drawables(CoilLayout::Single, true).as_slice(),
            "tests/img/rectangular_wo_opening.png",
            None,
        );

        compare_to_reference(
            slot.drawables(CoilLayout::Single, false).as_slice(),
            "tests/img/rectangular_wo_opening.png",
            None,
        );
    }
}

#[test]
fn test_slices() {
    let slot = RectangularSlot::new(
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .expect("valid inputs");
    {
        let slices = slot.slices(2);

        let slice_1 = &slices[0];
        approx::assert_abs_diff_eq!(slice_1.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.ymin(), 10.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.ymax(), 20.0e-3, epsilon = 1e-6);

        let slice_2 = &slices[1];
        approx::assert_abs_diff_eq!(slice_2.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.ymin(), 0.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.ymax(), 10.0e-3, epsilon = 1e-6);
    }
    {
        let slices = slot.slices(4);

        let slice_1 = &slices[0];
        approx::assert_abs_diff_eq!(slice_1.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.ymin(), 15.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_1.ymax(), 20.0e-3, epsilon = 1e-6);

        let slice_2 = &slices[1];
        approx::assert_abs_diff_eq!(slice_2.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.ymin(), 10.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_2.ymax(), 15.0e-3, epsilon = 1e-6);

        let slice_3 = &slices[2];
        approx::assert_abs_diff_eq!(slice_3.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_3.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_3.ymin(), 5.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_3.ymax(), 10.0e-3, epsilon = 1e-6);

        let slice_4 = &slices[3];
        approx::assert_abs_diff_eq!(slice_4.xmin(), -2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_4.xmax(), 2.5e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_4.ymin(), 0.0e-3, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(slice_4.ymax(), 5.0e-3, epsilon = 1e-6);
    }
}

#[test]
fn test_slot_layer_contour() {
    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(1.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

    let outline = 0.001 * (19.0 + 19.0 + 3.0 + 2.0); // two times body height + slot bottom + slot top

    assert!(slot.is_open());
    approx::assert_abs_diff_eq!(
        slot.outline().length(),
        outline + 2.0 * slot.opening_height().get::<meter>(),
        epsilon = 1e-6
    );

    // Sum up the partial slot outlines
    approx::assert_abs_diff_eq!(
        slot.layer_contour(0, &CoilLayout::Single).length(),
        outline,
        epsilon = 1e-6
    );

    let pt1 = slot
        .layer_contour(0, &CoilLayout::DoubleHorizontal)
        .length();
    let pt2 = slot
        .layer_contour(1, &CoilLayout::DoubleHorizontal)
        .length();
    assert!(pt1 == pt2); // Both outlines cover one half of the slot
    approx::assert_abs_diff_eq!(pt1 + pt2, outline, epsilon = 1e-6);

    let pt1 = slot.layer_contour(0, &CoilLayout::DoubleVertical).length();
    let pt2 = slot.layer_contour(1, &CoilLayout::DoubleVertical).length();
    assert!(pt1 > pt2); // pt1 is much larger since it includes the slot bottom
    approx::assert_abs_diff_eq!((pt1 + pt2), outline, epsilon = 1e-6);

    let pt1 = slot.layer_contour(0, &CoilLayout::Quadruple).length();
    let pt2 = slot.layer_contour(1, &CoilLayout::Quadruple).length();
    let pt3 = slot.layer_contour(2, &CoilLayout::Quadruple).length();
    let pt4 = slot.layer_contour(3, &CoilLayout::Quadruple).length();
    approx::assert_abs_diff_eq!(pt1 + pt2 + pt3 + pt4, outline, epsilon = 1e-6);

    let pt1 = slot
        .layer_contour(0, &CoilLayout::MultiVertical(4))
        .length();
    let pt2 = slot
        .layer_contour(1, &CoilLayout::MultiVertical(4))
        .length();
    let pt3 = slot
        .layer_contour(2, &CoilLayout::MultiVertical(4))
        .length();
    let pt4 = slot
        .layer_contour(3, &CoilLayout::MultiVertical(4))
        .length();
    approx::assert_abs_diff_eq!(pt1 + pt2 + pt3 + pt4, outline, epsilon = 1e-6);
}

#[test]
fn test_current_displacement_coefficients() {
    let frequency = Frequency::new::<hertz>(50.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m
    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);

    {
        let slot =
            RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

        let alpha = (TAU * frequency)
            / phase_velocity(
                frequency,
                el_conductivity,
                rel_permeability * *VACUUM_PERMEABILITY,
            );
        let alpha_height = f64::from(alpha * slot.height());
        approx::assert_abs_diff_eq!(alpha_height, 1.709211, epsilon = 1e-6);

        // Use the analytic approach
        let coeffs = slot.current_displacement_coefficients(100).eval(
            frequency,
            el_conductivity,
            rel_permeability,
        );
        approx::assert_abs_diff_eq!(coeffs.resistance, 1.576327, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(coeffs.inductance, 0.840828, epsilon = 1e-6);
    }
    {
        // Use the numeric approach
        let slot =
            RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

        {
            // Perform the same calculation twice to check that the internal
            // buffers of the coefficient calculator do not influence the next
            // calculation
            let mut calc = slot.current_displacement_coefficients(100);

            {
                let coeffs = calc.eval(frequency, el_conductivity, rel_permeability);
                approx::assert_abs_diff_eq!(coeffs.resistance, 1.576327, epsilon = 1e-6);
                approx::assert_abs_diff_eq!(coeffs.inductance, 0.840828, epsilon = 1e-6);
            }
            {
                let coeffs = calc.eval(frequency, el_conductivity, rel_permeability);
                approx::assert_abs_diff_eq!(coeffs.resistance, 1.576327, epsilon = 1e-6);
                approx::assert_abs_diff_eq!(coeffs.inductance, 0.840828, epsilon = 1e-6);
            }
        }
    }
}

#[test]
fn test_self_inductance_leakage_coefficient() {
    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

    // Analytical formula of the slot leakage coefficient: In the layer area, the
    // coefficient is h / 3b, above that, it is h / b.

    // Single-layer winding
    let coil_layout = CoilLayout::Single;
    approx::assert_abs_diff_eq!(
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        f64::from((height - opening_height) / (3.0 * width)),
        epsilon = 1e-6
    );

    // Double-layer winding
    let coil_layout = CoilLayout::DoubleHorizontal;
    approx::assert_abs_diff_eq!(
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        f64::from((height - opening_height) / (3.0 * width)),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.self_inductance_leakage_coefficient(1, &coil_layout),
        f64::from((height - opening_height) / (3.0 * width)),
        epsilon = 1e-6
    );

    let coil_layout = CoilLayout::DoubleVertical;
    let layer_height = 0.5 * (slot.height() - slot.opening_height());
    approx::assert_abs_diff_eq!(
        slot.self_inductance_leakage_coefficient(0, &coil_layout),
        f64::from(layer_height / (3.0 * width) + layer_height / width),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.self_inductance_leakage_coefficient(1, &coil_layout),
        f64::from(layer_height / (3.0 * width)),
        epsilon = 1e-6
    );
}

#[test]
fn test_mutual_inductance_leakage_coefficient() {
    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

    // Analytical formula of the self inductance slot leakage coefficient: In the
    // layer area, the coefficient is h / 3b, above that, it is h / b.
    // Analytical formula of the mutual inductance slot leakage coefficient: In the
    // layer area, the coefficient is h / 2b, above that, it is h / b.

    // Double-layer winding
    let coil_layout = CoilLayout::DoubleHorizontal;
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        f64::from((height - opening_height) / (3.0 * width)),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        f64::from((height - opening_height) / (3.0 * width)),
        epsilon = 1e-6
    );

    let coil_layout = CoilLayout::DoubleVertical;
    let layer_height = 0.5 * (slot.height() - slot.opening_height());
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        f64::from(layer_height / (2.0 * width)),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        f64::from(layer_height / (2.0 * width)),
        epsilon = 1e-6
    );

    let coil_layout = CoilLayout::MultiVertical(3);
    let layer_height = (slot.height() - slot.opening_height()) / 3.0;
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        f64::from(layer_height / (2.0 * width) + layer_height / width),
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        f64::from(layer_height / (2.0 * width) + layer_height / width),
        epsilon = 1e-6
    );
}

#[test]
fn test_leakage_coefficient_matrix() {
    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

    let coil_layout = CoilLayout::DoubleHorizontal;
    let matrix = slot.leakage_coefficient_matrix(&coil_layout);
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 0, &coil_layout),
        matrix[(0, 0)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        matrix[(1, 0)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        matrix[(0, 1)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 1, &coil_layout),
        matrix[(1, 1)],
        epsilon = 1e-6
    );

    let coil_layout = CoilLayout::DoubleVertical;
    let matrix = slot.leakage_coefficient_matrix(&coil_layout);
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 0, &coil_layout),
        matrix[(0, 0)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &coil_layout),
        matrix[(1, 0)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &coil_layout),
        matrix[(0, 1)],
        epsilon = 1e-6
    );
    approx::assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 1, &coil_layout),
        matrix[(1, 1)],
        epsilon = 1e-6
    );
}

#[test]
fn test_deserialize() {
    let yaml = indoc! {"
        width: 1.0 mm
        opening_width: 1.0 mm
        height: 1.0 mm
        opening_height: 0.5 mm
        consider_tooth_tip_leakage: true
        "};
    assert!(serde_yaml::from_str::<RectangularSlot>(&yaml).is_ok());
}

#[test]
fn test_deserialize_with_bad_parameters() {
    {
        let yaml = indoc! {"
            width: -1.0 mm                   # <== WIDTH MUST BE POSITIVE
            opening_width: 1.0 mm
            height: 1.0 mm
            opening_height: 0.5 mm
            consider_tooth_tip_leakage: true
            "};
        assert!(serde_yaml::from_str::<RectangularSlot>(&yaml).is_err());
    }

    {
        let yaml = indoc! {"
            width: 1.0 mm
            opening_width: 1.0 mm
            height: 1.0 mm
            opening_height: 2.0  mm           # <== OPENING_HEIGHT MUST SMALLER THAN HEIGHT
            consider_tooth_tip_leakage: true
            "};
        assert!(serde_yaml::from_str::<RectangularSlot>(&yaml).is_err());
    }
}
