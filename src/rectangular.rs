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
    #[cfg_attr(feature = "serde", serde(skip))]
    outline: Polysegment,
}

impl RectangularSlot {
    /// Create a new instance of `RectangularSlot`
    pub fn new(
        width: Length,          // Slot width (equal over slot height)
        opening_width: Length,  // Width of the slot opening
        height: Length,         // Total slot height (including slot opening)
        opening_height: Length, // Height of the slot opening
        consider_tooth_tip_leakage: bool,
    ) -> Result<Self, crate::error::Error> {
        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero <= opening_width)?;
        compare_variables!(val zero < width)?;
        compare_variables!(val zero < height)?;
        compare_variables!(opening_height < height)?;

        let outline = {
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
        if let Some(intersection) = outline
            .intersections_polysegment_par(&outline, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
            .find_map_any(|v| Some(v))
        {
            return Err(crate::error::Error::OutlineIntersection {
                intersection,
                outline,
            });
        }

        return Ok(RectangularSlot {
            width,
            opening_width,
            height,
            opening_height,
            consider_tooth_tip_leakage,
            outline,
        });
    }

    pub fn width(&self) -> Length {
        return self.width;
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for RectangularSlot {
    fn outline(&self) -> Cow<'_, Polysegment> {
        return Cow::Borrowed(&self.outline);
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

    fn leakage_coefficient_tooth_tip(&self, magnetic_air_gap: Length) -> f64 {
        if self.consider_tooth_tip_leakage {
            crate::slot::leakage_coefficient_tooth_tip(self.opening_width(), magnetic_air_gap)
        } else {
            0.0
        }
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
        }

        let s = RectangularSlotSerde::deserialize(deserializer)?;
        return RectangularSlot::new(
            s.width,
            s.opening_width,
            s.height,
            s.opening_height,
            s.consider_tooth_tip_leakage,
        )
        .map_err(serde::de::Error::custom);
    }
}
