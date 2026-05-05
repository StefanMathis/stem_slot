/**
! The `RectangularSlot` struct represents a semi-closed slot which has a rectangular shape.
It can be seen as a special case of `SlotTrapezoidSemi`. For this particular slot shape,
the current displacement can be expressed analytically.
*/
use std::borrow::Cow;

use rayon::prelude::*;

use planar_geo::prelude::*;

use compare_variables::compare_variables;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_material::prelude::*;

use crate::coil_layout::CoilLayout;
use crate::current_displacement::CurrentDisplacementCoefficients;
use crate::current_displacement::{
    current_displacement_coefficients_analytic, current_displacement_coefficients_numeric,
};
use crate::slot::Slot;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RectangularSlot {
    width: Length,          // Slot width (equal over slot height)
    opening_width: Length,  // Width of the slot opening
    height: Length,         // Total slot height (including slot opening)
    opening_height: Length, // Height of the slot opening
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                             * diagram 3.7.2 of [MVP08] or not. */
    analytic_current_displacement: bool, /* Whether to calculate the current displacement
                                          * coefficients analytically or numerically */
    #[cfg_attr(feature = "serde", serde(skip))]
    polysegment: Polysegment,
}

impl RectangularSlot {
    /// Create a new instance of `RectangularSlot`
    pub fn new(
        width: Length,          // Slot width (equal over slot height)
        opening_width: Length,  // Width of the slot opening
        height: Length,         // Total slot height (including slot opening)
        opening_height: Length, // Height of the slot opening
        consider_tooth_tip_leakage: bool,
        analytic_current_displacement: bool,
    ) -> Result<Self, crate::error::Error> {
        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < opening_width)?;
        compare_variables!(val zero < width)?;
        compare_variables!(val zero < height)?;
        compare_variables!(opening_height < height)?;

        let polysegment = {
            let mut pts = [[0.0; 2]; 8];

            pts[1] = [
                opening_width.get::<meter>() / 2.0,
                opening_height.get::<meter>(),
            ];
            pts[6] = [
                -opening_width.get::<meter>() / 2.0,
                opening_height.get::<meter>(),
            ];

            pts[2] = [width.get::<meter>() / 2.0, opening_height.get::<meter>()];
            pts[5] = [-width.get::<meter>() / 2.0, opening_height.get::<meter>()];

            pts[3] = [width.get::<meter>() / 2.0, height.get::<meter>()];
            pts[4] = [-width.get::<meter>() / 2.0, height.get::<meter>()];

            if opening_width > zero {
                pts[0] = [opening_width.get::<meter>() / 2.0, 0.0];
                pts[7] = [-opening_width.get::<meter>() / 2.0, 0.0];
                Polysegment::from_points(&pts)
            } else {
                Polysegment::from_points(&pts[1..6])
            }
        };

        // Assert that the outline does not intersect itself
        if let Some(intersection) = polysegment
            .intersections_polysegment_par(&polysegment, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
            .find_map_any(|v| Some(v))
        {
            return Err(crate::error::Error::OutlineIntersection {
                intersection,
                outline: polysegment,
            });
        }

        return Ok(RectangularSlot {
            width,
            opening_width,
            height,
            opening_height,
            consider_tooth_tip_leakage,
            analytic_current_displacement,
            polysegment,
        });
    }

    pub fn width(&self) -> Length {
        return self.width;
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for RectangularSlot {
    fn polysegment(&self) -> Cow<'_, Polysegment> {
        return Cow::Borrowed(&self.polysegment);
    }

    fn height(&self) -> Length {
        return self.height;
    }

    fn opening_width(&self) -> Length {
        return self.opening_width;
    }

    fn opening_height(&self) -> Length {
        return self.opening_height;
    }

    fn magnetic_opening_height(&self) -> Length {
        return self.opening_height;
    }

    /**
    Calculates the current displacement coefficients [kr, kx],
    where kr is the resistance increase coefficient and kx is the leakage inductance reduction coefficient.
    Depending on the value of `analytic_current_displacement`, either an analytical or a numerical solution is calculated.

    # OptimizationParameters
    - &self: Slot instance
    - frequency: Frequency of the electrical current
    - el_conductivity: Electrical conductivity
    - rel_permeability: Relative material permeability

    ```
    use approx;
    use slot::{RectangularSlot, Slot};
    use core::f64::consts::PI;
    use uom::si::f64::*;
    use uom::si::{
        electrical_conductivity::siemens_per_meter, length::millimeter, frequency::hertz,
    };

    let frequency = Frequency::new::<hertz>(50.0);
    let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6); // electrical conductivity of aluminium is about 37*1e6 S / m

    let rel_permeability = 1.0; // Relative permeability of aluminium is about 1

    let opening_height = Length::new::<millimeter>(1.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);

    // Use the analytic approach
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true, true).unwrap();
    let coeffs = slot.current_displacement_coefficients(frequency, el_conductivity, rel_permeability);
    approx::assert_abs_diff_eq!(coeffs.resistance_coefficient, 1.575749, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance_coefficient, 0.838612, epsilon = 1e-6);

    // Use the numeric approach
    let slot = RectangularSlot::new(width, opening_width, height, opening_height, true, false).unwrap();
    let coeffs = slot.current_displacement_coefficients(frequency, el_conductivity, rel_permeability);
    approx::assert_abs_diff_eq!(coeffs.resistance_coefficient, 1.313769, epsilon = 1e-6);
    approx::assert_abs_diff_eq!(coeffs.inductance_coefficient, 0.592730, epsilon = 1e-6);
    ```
    */
    fn current_displacement_coefficients(
        &self,
        frequency: Frequency,
        el_conductivity: ElectricalConductivity,
        rel_permeability: f64,
    ) -> CurrentDisplacementCoefficients {
        if self.analytic_current_displacement {
            return current_displacement_coefficients_analytic(
                self.height(),
                frequency,
                el_conductivity,
                rel_permeability,
            );
        } else {
            let shapes = self.shapes(CoilLayout::Single, true);
            let [x_ul, y_ul, x_lr, y_lr] = self.slices(100, &shapes[0].contour());

            let mut width: Vec<Length> = Vec::with_capacity(x_ul.len());
            let mut height: Vec<Length> = Vec::with_capacity(x_ul.len());
            for ii in 0..x_ul.len() {
                width.push(Length::new::<meter>(x_lr[ii] - x_ul[ii]));
                height.push(Length::new::<meter>(y_ul[ii] - y_lr[ii]));
            }
            return current_displacement_coefficients_numeric(
                width.as_slice(),
                height.as_slice(),
                frequency,
                el_conductivity,
                rel_permeability,
            );
        }
    }

    fn consider_tooth_tip_leakage(&self) -> bool {
        return self.consider_tooth_tip_leakage;
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RectangularSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use dyn_quantity::deserialize_quantity;

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct RectangularSlotSerde {
            #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
            width: Length,
            #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
            opening_width: Length,
            #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
            height: Length,
            #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
            opening_height: Length,
            consider_tooth_tip_leakage: bool,
            analytic_current_displacement: bool,
        }

        let s = RectangularSlotSerde::deserialize(deserializer)?;
        return RectangularSlot::new(
            s.width,
            s.opening_width,
            s.height,
            s.opening_height,
            s.consider_tooth_tip_leakage,
            s.analytic_current_displacement,
        )
        .map_err(serde::de::Error::custom);
    }
}
