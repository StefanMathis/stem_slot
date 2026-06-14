use stem_slot::{current_displacement::phase_velocity, prelude::*};

#[test]
fn test_phase_velocity() {
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m

    let frequency = Frequency::new::<hertz>(1.0);
    let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
    approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 0.519875, epsilon = 1e-6);

    let frequency = Frequency::new::<hertz>(10.0);
    let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
    approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 1.643989, epsilon = 1e-6);

    let frequency = Frequency::new::<hertz>(50.0);
    let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
    approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 3.676073, epsilon = 1e-6);
}

#[test]
fn test_current_displacement_coefficients_analytic_rectangular() {
    let height = Length::new::<millimeter>(20.0); // 20 mm high bar
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m
    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    let coeffs = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        height,
        Frequency::new::<hertz>(50.0),
        el_conductivity,
        rel_permeability,
    );
    approx::assert_abs_diff_eq!(coeffs.resistance, 1.575749, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance, 0.838612, epsilon = 1e-6);

    let coeffs = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        height,
        Frequency::new::<hertz>(100.0),
        el_conductivity,
        rel_permeability,
    );
    approx::assert_abs_diff_eq!(coeffs.resistance, 2.383342, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance, 0.631493, epsilon = 1e-6);
}

#[test]
fn test_current_displacement_calculator() {
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);
    {
        // No slices
        let mut calculator = CurrentDisplacementCalculator::from_slice_dims([].into_iter());

        let coeffs = calculator.eval(Frequency::new::<hertz>(100.0), el_conductivity, 1.0);
        assert_eq!(coeffs.resistance, 1.0);
        assert_eq!(coeffs.inductance, 1.0);

        let coeffs = calculator.eval(Frequency::new::<hertz>(5.0), el_conductivity, 1.0);
        assert_eq!(coeffs.resistance, 1.0);
        assert_eq!(coeffs.inductance, 1.0);

        let coeffs = calculator.eval(Frequency::new::<hertz>(5.0), el_conductivity, 1000.0);
        assert_eq!(coeffs.resistance, 1.0);
        assert_eq!(coeffs.inductance, 1.0);
    }
}
