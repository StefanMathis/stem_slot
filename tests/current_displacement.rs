use stem_slot::{
    current_displacement::{current_displacement_coefficients_analytic, phase_velocity},
    prelude::*,
};

#[test]
fn test_test_phase_velocity() {
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
fn test_test_current_displacement_coefficients_analytic_rectangular() {
    let height = Length::new::<millimeter>(20.0); // 20 mm high bar
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m
    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    let coeffs = current_displacement_coefficients_analytic(
        height,
        Frequency::new::<hertz>(50.0),
        el_conductivity,
        rel_permeability,
    );
    approx::assert_abs_diff_eq!(coeffs.resistance_coefficient, 1.575749, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance_coefficient, 0.838612, epsilon = 1e-6);

    let coeffs = current_displacement_coefficients_analytic(
        height,
        Frequency::new::<hertz>(100.0),
        el_conductivity,
        rel_permeability,
    );
    approx::assert_abs_diff_eq!(coeffs.resistance_coefficient, 2.383342, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance_coefficient, 0.631493, epsilon = 1e-6);
}
