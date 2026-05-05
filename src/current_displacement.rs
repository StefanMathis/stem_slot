use num::Complex;
use std::f64::consts::{PI, TAU};
use stem_material::prelude::*;

/// Calculate the phase velocity α from frequency, conductivity and
/// permeability. This is also known as "Vier-Griechen-Formel" (four greeks
/// formulae) See https://de.wikipedia.org/wiki/Skin-Effekt.
pub fn phase_velocity(
    frequency: Frequency,
    el_conductivity: ElectricalConductivity,
    abs_permeability: MagneticPermeability,
) -> Velocity {
    let k = (PI * frequency * el_conductivity * abs_permeability).sqrt();
    return (TAU * frequency) / k;
}

/// Calculate the current displacement coefficients for resistance and leakage
/// inductance of a rectangular bar with height `h`. Only sinusoidal currents
/// are considered. The formulae is taken from [Hut18], 31.1d & 31.1e.
pub fn current_displacement_coefficients_analytic(
    height: Length,
    frequency: Frequency,
    el_conductivity: ElectricalConductivity,
    rel_permeability: f64,
) -> CurrentDisplacementCoefficients {
    let alpha = (TAU * frequency)
        / phase_velocity(
            frequency,
            el_conductivity,
            rel_permeability * *VACUUM_PERMEABILITY,
        );
    let alpha_height = f64::from(alpha * height);
    let kr = alpha_height * ((2.0 * alpha_height).sinh() + (2.0 * alpha_height).sin())
        / ((2.0 * alpha_height).cosh() - (2.0 * alpha_height).cos());
    let kx = 1.5 / alpha_height * ((2.0 * alpha_height).sinh() - (2.0 * alpha_height).sin())
        / ((2.0 * alpha_height).cosh() - (2.0 * alpha_height).cos());
    return CurrentDisplacementCoefficients {
        resistance_coefficient: kr,
        inductance_coefficient: kx,
    };
}

/// Generalized calculation of the current displacement coefficients according
/// to [Hut18], 31.2a & 31.2b. Only sinusoidal currents are considered.
/// The calculation is based on a separation of the slot into rectangular
/// slices. For structs which implement `Slot`, this can be done via the default
/// implementation of the `slices()` method. The width and height vectors can
/// then be derived by subtracting the upper left from the lower right corner
/// values.
///
/// - width: Width of all slices, starting from the slot bottom.
/// - height: Height of all slices, starting from the slot bottom.
pub fn current_displacement_coefficients_numeric(
    width: &[Length],
    height: &[Length],
    frequency: Frequency,
    el_conductivity: ElectricalConductivity,
    _rel_permeability: f64,
) -> CurrentDisplacementCoefficients {
    use uom::typenum::P2;

    let omega = TAU * frequency;
    let axial_length = Length::new::<meter>(1.0); // Normalized slot length of 1 m, doesn't influence the result because it cancels out

    let number_slices = width.len();

    // Calculate resistances
    let mut resistances: Vec<ElectricalResistance> = Vec::with_capacity(number_slices);
    let mut area = Area::new::<square_meter>(0.0);
    for (w, h) in width.iter().zip(height.iter()) {
        resistances.push(axial_length / (*w * *h * el_conductivity));
        area = area + *w * *h;
    }

    // Current distribution. The first value is arbitrarily set to 1 A
    let mut i: Vec<Complex<ElectricCurrent>> = Vec::with_capacity(number_slices);
    i.push(Complex::new(
        ElectricCurrent::new::<ampere>(1.0),
        ElectricCurrent::new::<ampere>(0.0),
    ));

    // Magnetic field strength
    let mut field_strength: Vec<Complex<MagneticFieldStrength>> = Vec::with_capacity(number_slices);

    // Helper value which represents the total current in all slices from 1 to k
    let mut i_sum = Complex::new(
        ElectricCurrent::new::<ampere>(0.0),
        ElectricCurrent::new::<ampere>(0.0),
    );

    // Total conductance of the parallel slices
    let mut total_conductance = ElectricalConductance::new::<siemens>(0.0); // total_conductance

    // Losses and magnetic field energy w/ current displacement
    let mut losses = Power::new::<watt>(0.0); // losses
    let mut magnetic_field_energy = Energy::new::<joule>(0.0); // magnetic_field_energy

    // Loop over the slices
    for ii in 0..number_slices {
        // Increaste the current sum with the value of the current slice
        i_sum = Complex::new(i_sum.re + i[ii].re, i_sum.im + i[ii].im);

        // See [Hut18], 31.2a
        if ii < number_slices - 1 {
            let coeff_r = resistances[ii] / resistances[ii + 1];
            let coeff_om = omega * *VACUUM_PERMEABILITY * axial_length * height[ii]
                / (resistances[ii + 1] * width[ii]);
            i.push(Complex::new(
                i[ii].re * coeff_r + i_sum.re * coeff_om,
                i[ii].im * coeff_r + i_sum.im * coeff_om,
            ));
        }
        field_strength.push(Complex::new(i_sum.re / width[ii], i_sum.im / width[ii]));

        // See [Hut18], 31.2b
        total_conductance = total_conductance + 1.0 / resistances[ii];
        losses = losses + resistances[ii] * (i[ii].re.powi(P2::new()) + i[ii].im.powi(P2::new()));
        magnetic_field_energy = magnetic_field_energy
            + 0.5
                * *VACUUM_PERMEABILITY
                * (field_strength[ii].re.powi(P2::new()) + field_strength[ii].im.powi(P2::new()))
                * height[ii]
                * width[ii]
                * axial_length
    }

    let i_square = i_sum.re.powi(P2::new()) + i_sum.im.powi(P2::new());

    // Calculate the losses with and w/o current displacement. The ratio is the
    // resistance coefficient
    let res_wo_cd = 1.0 / total_conductance; // R0
    let losses_wo_cd = res_wo_cd * i_square; // P_v0
    let kr = f64::from(losses / losses_wo_cd);

    // Calculate the magnetic energy with and w/o current displacement. The ratio is
    // the leakage inductance coefficient
    let mut cumulative_area = Area::new::<square_meter>(0.0); // A_μ
    let mut magnetic_field_energy_wo_cd = Energy::new::<joule>(0.0); // W_m0
    let i_over_area = i_square.sqrt() / area;
    for ii in 0..number_slices {
        let area_current_slice = height[ii] * width[ii];
        cumulative_area = cumulative_area + area_current_slice;
        let field_strength_wo_cd = i_over_area * cumulative_area / width[ii]; // H0
        magnetic_field_energy_wo_cd = magnetic_field_energy_wo_cd
            + 0.5
                * *VACUUM_PERMEABILITY
                * field_strength_wo_cd.powi(P2::new())
                * area_current_slice
                * axial_length;
    }

    // Multiply with constant values. This step can be left out, since the values
    // are canceled out when calculating the leakage coefficient
    // magnetic_field_energy *= 0.5*MU0*axial_length;
    // magnetic_field_energy_wo_cd *= 0.5*MU0*axial_length;
    let kx = f64::from(magnetic_field_energy / magnetic_field_energy_wo_cd);
    return CurrentDisplacementCoefficients {
        resistance_coefficient: kr,
        inductance_coefficient: kx,
    };
}

#[derive(Clone, Debug)]
pub struct CurrentDisplacementCoefficients {
    pub resistance_coefficient: f64,
    pub inductance_coefficient: f64,
}

impl Default for CurrentDisplacementCoefficients {
    fn default() -> Self {
        return CurrentDisplacementCoefficients {
            resistance_coefficient: 1.0,
            inductance_coefficient: 1.0,
        };
    }
}
