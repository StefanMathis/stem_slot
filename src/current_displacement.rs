/*!
A module for calculating current displacement factors.

In massive conductors, an alternating current is not evenly spread across the
cross-section, but instead is "displaced" by its own magnetic field. This
displacement reduces the effective cross section of the conductor, resulting in
an increased resistance and reduced inductance. The effect depends on the slot /
conductor geometry as well as on external factors like the frequency of the
alternating current, the electric conductivity and the relative permeability of
the conductor. Within the context of this module, the massive conductor is
always assumed to fill the entire slot.

This module defines the [`CurrentDisplacementCoefficients`] struct, which holds
coefficients describing the effect of current displacement on the resistance
and the self-inductance of a conductor. To calculate these coefficients for
arbitrary conductor / slot geometries, the [`CurrentDisplacementCalculator`] can
be used.
 */
use num::Complex;
use rayon::prelude::*;
use std::f64::consts::{PI, TAU};
use stem_material::prelude::*;

use crate::slot::Slot;

/**
Returns the phase velocity `α` at which a magnetic field enters a conductor
carrying an AC current.

If an electric conductor with the given `el_conductivity` and `permeability`
carries an AC current with the given `frequency`, the speed at which a magnetic
field can enter the conductor from the outside is called the "phase velocity"
`α`. It can be calculated with the "four greeks formulae":

`α = ω / sqrt(0.5 * ω * μ * κ)`

with `ω = 2 * π * frequency`, `κ = el_conductivity` and `μ = permeability`.

# Examples

```
use approx;
use stem_slot::current_displacement::phase_velocity;
use stem_slot::prelude::*;

 // electrical conductivity of aluminium is about 37*1e6 S / m
let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);

let frequency = Frequency::new::<hertz>(1.0);
let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 0.519875, epsilon = 1e-6);

let frequency = Frequency::new::<hertz>(10.0);
let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 1.643989, epsilon = 1e-6);

let frequency = Frequency::new::<hertz>(50.0);
let alpha = phase_velocity(frequency, el_conductivity, *VACUUM_PERMEABILITY);
approx::assert_abs_diff_eq!(alpha.get::<meter_per_second>(), 3.676073, epsilon = 1e-6);
```
 */
pub fn phase_velocity(
    frequency: Frequency,
    el_conductivity: ElectricalConductivity,
    permeability: MagneticPermeability,
) -> Velocity {
    let k = (PI * frequency * el_conductivity * permeability).sqrt();
    return (TAU * frequency) / k;
}

/**
Current displacement factors for resistance and inductance.

In massive conductors, an alternating current is not evenly spread across the
cross-section, but instead is "displaced" by its own magnetic field. This
displacement reduces the effective cross section of the conductor, resulting in
an increased resistance and reduced conductor leakage inductance.

This effect can be modeled by factors for resistance and inductance which are
multiplied with the DC resistance and inductance to get the effective values
for the AC current as shown in /[1/], section 5.3.2.

>/[1/] Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
Maschinen, 6th edition (2008), Wiley-VCH, Weinheim
 */
#[derive(Clone, Debug)]
pub struct CurrentDisplacementCoefficients {
    /// Multiply the DC resistance with this value to get the effective
    /// AC resistance. Equals `krn` from equation (5.3.23) of /[1/].
    pub resistance: f64,
    /// Multiply the DC conductor leakage inductance with this value to get the
    /// effective AC leakage inductance. Equals `kxn` from equation (5.3.29) of
    /// /[1/].
    pub inductance: f64,
}

impl CurrentDisplacementCoefficients {
    /**
    Returns the coefficients for a rectangular, open slot completely filled
    by an conductor.

    For a rectangular, open slot with the given `height` which is filled by
    a single conductor with the specified `el_conductivity` and
    `rel_permeability`, the current displacement coefficients for a current with
    the given `frequency` can be calculated exactly, as shown in section 5.3.2
    of _Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    Maschinen, 6th edition (2008), Wiley-VCH, Weinheim_.

    # Examples

    ```
    use approx;
    use stem_slot::current_displacement::CurrentDisplacementCoefficients;
    use stem_slot::prelude::*;

    let height = Length::new::<millimeter>(10.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);

    // DC coefficients are 1
    let coeffs = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        height, Frequency::new::<hertz>(0.0), el_conductivity, 1.05
    );
    approx::assert_abs_diff_eq!(coeffs.resistance, 1.0, epsilon=1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance, 1.0, epsilon=1e-6);

    // AC coefficients
    let coeffs = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        height, Frequency::new::<hertz>(50.0), el_conductivity, 1.05
    );
    approx::assert_abs_diff_eq!(coeffs.resistance, 1.05113, epsilon=1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance, 0.98541, epsilon=1e-6);
    ```
     */
    pub fn from_rectangular_open_slot(
        height: Length,
        frequency: Frequency,
        el_conductivity: ElectricalConductivity,
        rel_permeability: f64,
    ) -> Self {
        if frequency.get::<hertz>() == 0.0 {
            return CurrentDisplacementCoefficients::default();
        }

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
            resistance: kr,
            inductance: kx,
        };
    }
}

impl Default for CurrentDisplacementCoefficients {
    fn default() -> Self {
        return CurrentDisplacementCoefficients {
            resistance: 1.0,
            inductance: 1.0,
        };
    }
}

/**
A calculator for the [`CurrentDisplacementCoefficients`] of a massive conductor
filling a slot.

# Overview

This struct holds a couple of buffers needed to estimate the
[`CurrentDisplacementCoefficients`] of a massive conductor numerically. When
calculating the coefficients repeatedly for different physical parameters
(electrical conductivity and permeability of the conductor, frequency of the
current), it is therefore not necessary to allocate these buffers repeatedly.
Instead, they are created once within the constructor (
[`CurrentDisplacementCalculator::new`] or
[`CurrentDisplacementCalculator::from_slice_dims`]) and are then reused when
calculating the coefficients with [`CurrentDisplacementCalculator::eval`].

# Physical background

If a slot is completely filled by a single massive conductor (1 turn) and the
core material can be treated as magnetically superconducting (permeability much
higher than that of air, usually true for ferromagnetic materials), the current
displacement coefficients can be calculated numerically by separating the slot
into vertically stacked [slices](Slot::slices), which are treated as parallel
conductors.
*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Numeric current displacement calculation][current_displacement_calculator]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "current_displacement_calculator",
        "docs/img/current_displacement_calculator.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
By using the formulae in [1], section 5.3.2, the (complex) current in the nth
conductor / slice can be calculated as

```text
I(n) = I(n-1) * R(n-1)/R(n) + j * ω * μ * slice_height[n] / (R(n) * slice_width[n]) * sum(I(0) ... I(n))
```

The individual slice resistances `R(n)` can be calculated from the slice
dimensions and the conductivity. By setting the current `I(1)` to an arbitrary
value (e.g. 1 A + j 0 A), the current distribution across the entire stack of
slices / across the conductor can be found.

From that, it is possible to derive
[`CurrentDisplacementCoefficients::resistance`] as the ratio between the sum
of losses in the indivual conductors and the DC losses:

```text
kr = (sum(R * I²)) / (sum(R) * sum(I)²)
```

Correspondingly, [`CurrentDisplacementCoefficients::inductance`] can be found
as the ratio between the sum of the magnetic energy in the slices and the DC
magnetic energy. See [1], section 5.3.2 for details.

>[1] Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
Maschinen, 6th edition (2008), Wiley-VCH, Weinheim

As discussed in [`Slot::slices`], it is evident that a higher number of slices
offers a more granular calculation and therefore a higher precision at the cost
of more CPU operations. For the special case of a rectangular bar conductor,
it is also possible to find a closed analytic solution (see
[`CurrentDisplacementCoefficients::from_rectangular_open_slot`]). This allows
for a comparison between the numeric and the analytic approach:
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Comparison analytic and numeric current displacement coefficients][current_displacement_coeffs_comp]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "current_displacement_coeffs_comp",
        "docs/img/current_displacement_coeffs_comp.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
#[derive(Clone, Debug)]
pub struct CurrentDisplacementCalculator {
    area: Area,
    slices: Vec<Slice>,
}

impl CurrentDisplacementCalculator {
    /**
    Creates a new instance of `Self` by dividing the `slot` into multiple slices
    using [`Slot::slices`] with the specified `min_num_slices`.

    This is essentially a convenience wrapper around
    [`CurrentDisplacementCalculator::from_slice_dims`].
     */
    pub fn new<S: Slot + ?Sized>(slot: &S, min_num_slices: usize) -> Self {
        let bbs = slot.slices(min_num_slices);
        return Self::from_slice_dims(bbs.into_iter().map(|bb| {
            let height = Length::new::<meter>(bb.height());
            let width = Length::new::<meter>(bb.width());
            [height, width]
        }));
    }

    /**
    Creates a new instance of `Self` from the given slice dimensions
    `[height, width]`.

    The iterator must start at the slot bottom and return the slices in order
    up to the slot top / air gap opening.
    */
    pub fn from_slice_dims<I: Iterator<Item = [Length; 2]>>(slice_dims: I) -> Self {
        let (mut capacity, upper) = slice_dims.size_hint();
        if let Some(u) = upper {
            capacity = u;
        }
        let mut slices = Vec::with_capacity(capacity);
        let mut area = Area::new::<square_meter>(0.0);
        for [height, width] in slice_dims {
            slices.push(Slice {
                height,
                width,
                resistance: Default::default(),
                current: Default::default(),
                magnetic_field_strength: Default::default(),
            });
            area += height * width;
        }

        // The first current value is arbitrarily set to 1 A. THe value does not matter,
        // as it cancels out in eval anyway.
        if let Some(s) = slices.get_mut(0) {
            s.current = Complex::new(
                ElectricCurrent::new::<ampere>(1.0),
                ElectricCurrent::new::<ampere>(0.0),
            );
        }

        return Self { area, slices };
    }

    /**
    Returns the [`CurrentDisplacementCoefficients`] for the specified current
    `frequency`, `el_conductivity` and `rel_permeability` of the conductor.

    This method modifies the internal buffers and therefore uses `&mut self`.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(5.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .expect("valid inputs");
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);
    let rel_permeability = 1.0;

    // Use 10 slices for the numeric calculation
    let mut calculator = slot.current_displacement_coefficients(10);

    // Now use the calculator repeatedly for currents of different frequencies
    // without allocations
    let coeffs50 = calculator.eval(Frequency::new::<hertz>(50.0), el_conductivity, rel_permeability);
    assert_abs_diff_eq!(coeffs50.resistance, 1.566, epsilon = 1e-3);
    assert_abs_diff_eq!(coeffs50.inductance, 0.861, epsilon = 1e-3);

    let coeffs100 = calculator.eval(Frequency::new::<hertz>(100.0), el_conductivity, rel_permeability);
    assert_abs_diff_eq!(coeffs100.resistance, 2.348, epsilon = 1e-3);
    assert_abs_diff_eq!(coeffs100.inductance, 0.684, epsilon = 1e-3);

    // Compare with analytical solution
    let coeffs = CurrentDisplacementCoefficients::from_rectangular_open_slot(
        slot.height(),
        Frequency::new::<hertz>(50.0),
        el_conductivity,
        rel_permeability
    );
    assert_abs_diff_eq!(coeffs.resistance, 1.576, epsilon = 1e-3);
    assert_abs_diff_eq!(coeffs.inductance, 0.839, epsilon = 1e-3);
    ```
     */
    pub fn eval(
        &mut self,
        frequency: Frequency,
        el_conductivity: ElectricalConductivity,
        rel_permeability: f64,
    ) -> CurrentDisplacementCoefficients {
        use uom::typenum::P2;

        // Angular electric frequency
        let omega = TAU * frequency;

        // Normalized slot length of 1 m, doesn't influence the result because it
        // cancels out
        let axial_length = Length::new::<meter>(1.0);

        // Populate resistances
        self.slices.par_iter_mut().for_each(|s| {
            s.resistance = axial_length / (s.width * s.height * el_conductivity);
        });

        // Total conductance of the parallel slices
        let mut total_conductance = ElectricalConductance::new::<siemens>(0.0);

        // Losses and magnetic field energy w/ current displacement
        let mut losses = Power::new::<watt>(0.0);
        let mut magnetic_field_energy = Energy::new::<joule>(0.0);

        // Helper value which represents the total current in all slices
        let mut i_sum = Complex::new(
            ElectricCurrent::new::<ampere>(0.0),
            ElectricCurrent::new::<ampere>(0.0),
        );

        // Loop over the slices
        for ii in 0..self.slices.len() {
            // Increase the current sum with the value of the current slice
            i_sum = Complex::new(
                i_sum.re + self.slices[ii].current.re,
                i_sum.im + self.slices[ii].current.im,
            );

            // See [Hut18], 31.2a
            if ii < self.slices.len() - 1 {
                let coeff_r = self.slices[ii].resistance / self.slices[ii + 1].resistance;
                let coeff_om = omega
                    * rel_permeability
                    * *VACUUM_PERMEABILITY
                    * axial_length
                    * self.slices[ii].height
                    / (self.slices[ii + 1].resistance * self.slices[ii].width);
                self.slices[ii + 1].current = Complex::new(
                    self.slices[ii].current.re * coeff_r - i_sum.im * coeff_om,
                    self.slices[ii].current.im * coeff_r + i_sum.re * coeff_om,
                );
            }

            self.slices[ii].magnetic_field_strength = Complex::new(
                i_sum.re / self.slices[ii].width,
                i_sum.im / self.slices[ii].width,
            );

            // See [Hut18], 31.2b
            total_conductance = total_conductance + 1.0 / self.slices[ii].resistance;
            losses = losses
                + self.slices[ii].resistance
                    * (self.slices[ii].current.re.powi(P2::new())
                        + self.slices[ii].current.im.powi(P2::new()));
            magnetic_field_energy = magnetic_field_energy
                + 0.5
                    * *VACUUM_PERMEABILITY
                    * (self.slices[ii].magnetic_field_strength.re.powi(P2::new())
                        + self.slices[ii].magnetic_field_strength.im.powi(P2::new()))
                    * self.slices[ii].height
                    * self.slices[ii].width
                    * axial_length
        }

        let i_square = i_sum.re.powi(P2::new()) + i_sum.im.powi(P2::new());

        // Calculate the losses with and w/o current displacement. The ratio is the
        // resistance coefficient
        let res_wo_cd = 1.0 / total_conductance; // R0
        let losses_wo_cd = res_wo_cd * i_square; // P_v0
        let kr = f64::from(losses / losses_wo_cd);

        // Calculate the magnetic energy with and w/o current displacement. The ratio is
        // the leakage inductance coefficient.
        let mut cumulative_area = Area::new::<square_meter>(0.0); // A_μ
        let mut magnetic_field_energy_wo_cd = Energy::new::<joule>(0.0); // W_m0
        let i_over_area = i_square.sqrt() / self.area;
        for slice in self.slices.iter() {
            let area_current_slice = slice.height * slice.width;
            cumulative_area = cumulative_area + area_current_slice;
            let field_strength_wo_cd = i_over_area * cumulative_area / slice.width; // H0
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
            resistance: kr,
            inductance: kx,
        };
    }
}

/// A single "slice" of the slot used in [`CurrentDisplacementCalculator`].
#[derive(Clone, Debug)]
struct Slice {
    height: Length,
    width: Length,
    resistance: ElectricalResistance,
    current: Complex<ElectricCurrent>,
    magnetic_field_strength: Complex<MagneticFieldStrength>,
}
