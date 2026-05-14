/*!
This crate creates the images used in the documentation of the stem_slot crate.
 */

use plotters::prelude::*;
use plotters::style::RelativeSize;
use stem_slot::prelude::*;
use stem_slot::slot::leakage_coefficient_tooth_tip;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    leakage_coefficient_tooth_tip_plot()?;
    current_displacement_plot()?;
    return Ok(());
}

fn leakage_coefficient_tooth_tip_plot() -> Result<(), Box<dyn std::error::Error>> {
    // General config
    // =========================================================================
    let size = (600, 400);
    let font_size_labels = 18;
    let font_size_ticks = 16;

    // Calculate value
    // =========================================================================

    let resolution = 1e-3;
    let xmin = 0.0;
    let xmax = 15.0;
    let capacity = ((xmax - xmin) / resolution) as usize;

    let mut ratio_opening_air_gap = Vec::with_capacity(capacity);
    let mut leakage_coeff = Vec::with_capacity(capacity);

    let mut x = xmin;
    while x <= xmax {
        ratio_opening_air_gap.push(x);
        leakage_coeff.push(leakage_coefficient_tooth_tip(
            Length::new::<millimeter>(x),
            Length::new::<millimeter>(1.0),
        ));
        x += resolution;
    }

    // Plotting
    // =========================================================================

    let file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../img/leakage_coefficient_tooth_tip.svg");
    let root = SVGBackend::new(&file_path, size).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(xmin..xmax, -0.2..1.0)?;

    chart
        .configure_mesh()
        .x_desc("opening_width / magnetic_air_gap")
        .y_desc("leakage_coefficient_tooth_tip")
        .axis_desc_style(("sans-serif", font_size_labels))
        .label_style(("sans-serif", font_size_ticks))
        .draw()?;

    // Interpolated data
    chart.draw_series(LineSeries::new(
        ratio_opening_air_gap
            .iter()
            .cloned()
            .zip(leakage_coeff.iter().cloned()),
        &BLUE,
    ))?;

    root.present().expect(&format!(
        "Unable to write result to file, please make sure you have write permissions for {}",
        file_path.display()
    ));

    return Ok(());
}

fn current_displacement_plot() -> Result<(), Box<dyn std::error::Error>> {
    // General config
    // =========================================================================
    let size = (800, 400);
    let font_size_labels = 18;
    let font_size_ticks = 16;
    let font_size_legend = 18;
    let font_size_caption = 20;

    // An open rectangular slot
    let slot = RectangularSlot::new(
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .expect("valid inputs");

    let frequency = Frequency::new::<hertz>(50.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);
    let rel_permeability = 1.0;

    let max_num_slices = 100;

    // Coefficients
    // =========================================================================

    let mut resistance_coeff = Vec::with_capacity(max_num_slices);
    let mut inductance_coeff = Vec::with_capacity(max_num_slices);
    let vec_num_slices: Vec<f64> = (1..max_num_slices).map(|x| x as f64).collect();

    for num_slices in 1..max_num_slices {
        let coeffs = slot.current_displacement_coefficients(num_slices).eval(
            frequency,
            el_conductivity,
            rel_permeability,
        );
        resistance_coeff.push(coeffs.resistance);
        inductance_coeff.push(coeffs.inductance);
    }

    let coeffs_analytical = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        slot.height(),
        frequency,
        el_conductivity,
        rel_permeability,
    );

    // Plotting
    // =========================================================================

    let file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../img/current_displacement_coeffs_comp.svg");
    let root = SVGBackend::new(&file_path, size).into_drawing_area();
    let (left, right) = root.split_horizontally(RelativeSize::Width(0.5));

    // Resistance coefficients
    // =========================================================================

    left.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&left)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Resistance coefficient", ("sans-serif", font_size_caption))
        .build_cartesian_2d(1.0..(max_num_slices as f64), 1.0..1.6)?;

    chart
        .configure_mesh()
        .x_desc("number of slices")
        .axis_desc_style(("sans-serif", font_size_labels))
        .label_style(("sans-serif", font_size_ticks))
        .draw()?;

    // Interpolated data
    chart
        .draw_series(LineSeries::new(
            vec_num_slices
                .iter()
                .cloned()
                .zip(resistance_coeff.iter().cloned()),
            &BLUE,
        ))?
        .label("Numerical solution")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart
        .draw_series(LineSeries::new(
            [1.0, max_num_slices as f64].iter().cloned().zip(
                [coeffs_analytical.resistance, coeffs_analytical.resistance]
                    .iter()
                    .cloned(),
            ),
            &RED,
        ))?
        .label("Analytical solution")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8)) // semi-transparent background
        .label_font(("sans-serif", font_size_legend))
        .position(SeriesLabelPosition::LowerRight) // position on the chart
        .draw()?;

    // Inductance coefficients
    // =========================================================================

    right.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&right)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Inductance coefficient", ("sans-serif", font_size_caption))
        .build_cartesian_2d(1.0..(max_num_slices as f64), 0.8..1.0)?;

    chart
        .configure_mesh()
        .x_desc("number of slices")
        .axis_desc_style(("sans-serif", font_size_labels))
        .label_style(("sans-serif", font_size_ticks))
        .draw()?;

    // Interpolated data
    chart
        .draw_series(LineSeries::new(
            vec_num_slices
                .iter()
                .cloned()
                .zip(inductance_coeff.iter().cloned()),
            &BLUE,
        ))?
        .label("Numerical solution")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart
        .draw_series(LineSeries::new(
            [1.0, max_num_slices as f64].iter().cloned().zip(
                [coeffs_analytical.inductance, coeffs_analytical.inductance]
                    .iter()
                    .cloned(),
            ),
            &RED,
        ))?
        .label("Analytical solution")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8)) // semi-transparent background
        .label_font(("sans-serif", font_size_legend))
        .position(SeriesLabelPosition::UpperRight) // position on the chart
        .draw()?;

    root.present().expect(&format!(
        "Unable to write result to file, please make sure you have write permissions for {}",
        file_path.display()
    ));

    return Ok(());
}
