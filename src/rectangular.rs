/*!
This module defines a [`RectangularSlot`] - a simple slot consisting of a
rectangular groove, possibly with a semi-closed or closed slot opening. See the
struct documentation for more.
 */

use std::borrow::Cow;

use rayon::prelude::*;

use planar_geo::prelude::*;

use compare_variables::compare_variables;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_material::prelude::*;

use crate::slot::Slot;

/**
A simple slot composed of two rectangles: winding area and slot opening area.

This slot type is the standard for linear motors, because it results in teeth of
constant thickness and can be easily wound. If the slot is open (opening width
equals total width), it is even possible to prewind coils and then simply push
them onto the teeth, which is especially useful for the efficient creation of
tooth-coil windings. For rotary motors, a rectangular slot results in trapezoid
teeth and therefore wastes space, but retains the other advantages and therefore
can also sometimes be found there.

# Geometry

A rectangular slot is defined by the following parameters:
- `width`: Width of the winding area. Must be positive (`width > 0 m`).
- `opening_width`: Width of the slot opening area. Must be positive
(`width > 0 m`).
- `height`: Total height of the slot (height of winding area + height of slot
opening). Must be larger than `opening_height` (`height > opening_height`).
- `opening_height`: Height of the slot opening. Must be positive
(`opening_height > 0 m`).
*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Rectangular slot definitions][cad_rectangular]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_rectangular", "docs/img/cad_rectangular.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
# Serialization and deserialization

This struct can directly be deserialized from the parameters specified above,
provided that the values are within the specified limits. Additionally, the
`consider_tooth_tip_leakage` flag also needs to be given
(see [`RectangularSlot::new`]). Its serialized form is that of a map as shown
below.

```
use approx::assert_abs_diff_eq;
use stem_slot::prelude::*;
use serde_yaml;

let str = indoc::indoc! {"
width: 8 mm
opening_width: 4 mm
height: 20 mm
opening_height: 2 mm
consider_tooth_tip_leakage: true
"};

let slot: RectangularSlot = serde_yaml::from_str(&str).expect("valid dimensions");
assert_abs_diff_eq!(slot.winding_area().get::<square_millimeter>(), 144.0, epsilon=1e-3);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RectangularSlot {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_height: Length,
    consider_tooth_tip_leakage: bool,
    #[cfg_attr(feature = "serde", serde(skip))] // Gets generated when deserializing
    outline: Polysegment,
}

impl RectangularSlot {
    /**
    Returns a new [`RectangularSlot`].

    The dimensions need to fulfill the value range constraints from the struct
    docstring. If `consider_tooth_tip_leakage` is set to true, the default
    implementation of [`Slot::leakage_coefficient_tooth_tip`] is used to
    calculate the leakage coefficient. Otherwise, the coefficient is set to 0.

    # Examples

    ```
    use stem_slot::prelude::*;

    // Valid input parameters
    let slot_tt_leakage = RectangularSlot::new(
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(4.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");
    assert_eq!(slot_tt_leakage.leakage_coefficient_tooth_tip(Length::new::<millimeter>(0.5)), -0.11);

    let slot_no_tt_leakage = RectangularSlot::new(
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(4.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        false,
    ).expect("valid inputs");
    assert_eq!(slot_no_tt_leakage.leakage_coefficient_tooth_tip(Length::new::<millimeter>(0.5)), 0.0);

    // Negative lengths
    assert!(RectangularSlot::new(
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(4.0),
        Length::new::<millimeter>(-20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).is_err());

    // Total height smaller than opening height
    assert!(RectangularSlot::new(
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(4.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(2.0),
        true,
    ).is_err());

    ```
     */
    pub fn new(
        width: Length,
        opening_width: Length,
        height: Length,
        opening_height: Length,
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

    /**
    Returns the winding area width of `self`.
     */
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

    fn area(&self) -> Area {
        return self.winding_area() + self.opening_height * self.opening_width;
    }

    fn winding_area(&self) -> Area {
        return self.width * (self.height - self.opening_height);
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
        struct RectangularSlotSerde {
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
