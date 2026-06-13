/*!
This module defines a [`SemiTrapezoidSlot`] - a trapezoid slot which is
semi-opened or even closed towards the air gap - as well as a couple of
"builder" structs which can be used to create a [`SemiTrapezoidSlot`]. See the
struct documentation for more.
 */

use approx::ulps_eq;
use compare_variables::{Comparison, ComparisonOperator, ComparisonValue, compare_variables};
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::{
    borrow::Cow,
    f64::consts::{FRAC_PI_2, PI, TAU},
};
use stem_material::prelude::*;

use crate::slot::slot_side_bottom_and_top_width_from_rot_core;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::slot::{BottomAngle, Slot, TopAngle};

/**
A trapezoid slot which is semi-closed or closed towards the air gap.

Semi-trapezoid slots are the standard slot on rotary motors, especially for
distributed winding. Thanks to the semi-closed slot opening, the air gap width
permeance does not change compared to the tooth area, leading to a small
effective air gap. When special winding technologies are used, the slot might
even be fully closed to minimize the permeance disturbance. The trapezoid form
allows the usage of parallel-sided teeth, optimizing the available space for the
winding.

# Geometry and constructors

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

Not all the parameters shown in the image are needed to unequivocally describe
the slot geometry. For example, defining four of the five height parameters
directly sets the value of the fifth. Therefore, this module defines a couple
of "builder" structs which represent different possible parameter sets. These
can be fallibly converted to a [`SemiTrapezoidSlot`] via their [`TryFrom`]
implementations:
- [`SemiTrapezoidWidthsAndHeightsBuilder`] (builder version of [`SemiTrapezoidSlot::new`])
- [`SemiTrapezoidAnglesSideHeightBuilder`]
- [`SemiTrapezoidWithoutSlopesBuilder`]
- [`SemiTrapezoidAnglesBottomHeightBuilder`]
- [`SemiTrapezoidAnglesTopHeightBuilder`]
- [`SemiTrapezoidAnglesBottomSideWidthBuilder`]
- [`SemiTrapezoidAnglesTopSideWidthBuilder`]
- [`SemiTrapezoidFromToothWidthRotBuilder`]
- [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder`]

```
use approx;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder;

let builder = SemiTrapezoidWithoutSlopesBuilder {
    bottom_width: Length::new::<millimeter>(8.0),
    opening_width: Length::new::<millimeter>(2.0),
    opening_height: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(20.0),
    slot_angle: PI / 18.0,
    top_radius: Length::new::<millimeter>(1.0),
    bottom_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(1.0),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid inputs");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 2.0, epsilon=1e-3);
```

The conversion fails if a parameter is out of bounds or if the resulting slot
outline intersects itself. The bounds of a parameter is specified in the field
docstring of the respective builder struct. Values which are closer than
[`DEFAULT_EPSILON`] to zero may be rounded to zero to prevent building failures
due to limited floating point precision.

Using structs instead of constructor functions makes it less likely to confuse
arguments, since the parameter name needs to be specified explicitly. For
convenience, there exists a constructor function [`SemiTrapezoidSlot::new`]
which internally creates an [`SemiTrapezoidWidthsAndHeightsBuilder`] and then
converts it.

# Serialization and deserialization

This struct can be directly deserialized from any of its "builder" structs (no
need for a tag). Its serialized form is that of the
[`SemiTrapezoidWidthsAndHeightsBuilder`] struct.

```
use approx;
use stem_slot::prelude::*;
use serde_yaml;

// Parameters of a SemiTrapezoidAnglesSideHeightBuilder
let str = indoc::indoc! {"
bottom_width: 8 mm
bottom_angle: 135 deg
top_angle: 135 deg
top_width: 5 mm
opening_width: 2 mm
opening_height: 2 mm
height: 20 mm
side_height: 16 mm
slot_angle: PI / 18
bottom_radius: 2 mm 
bottom_side_radius: 1 mm
top_radius: 1 mm
top_side_radius: 1 mm
opening_radius: 1 mm
consider_tooth_tip_leakage: true
"};

let slot: SemiTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 2.0, epsilon=1e-3);

// Parameters of a SemiTrapezoidWithoutSlopesBuilder
let str = indoc::indoc! {"
bottom_width: 8 mm
opening_width: 2 mm
opening_height: 2 mm
height: 20 mm
slot_angle: PI / 18
bottom_radius: 2 mm
top_radius: 1 mm
opening_radius: 1 mm
consider_tooth_tip_leakage: true
"};

let slot: SemiTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 2.0, epsilon=1e-3);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SemiTrapezoidSlot {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_side_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    top_side_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    top_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    side_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    top_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_side_radius: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    top_radius: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    top_side_radius: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_radius: Length,
    consider_tooth_tip_leakage: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    outline: Polysegment,
}

impl SemiTrapezoidSlot {
    /**
    Creates a new [`SemiTrapezoidSlot`].

    This is the function equivalent for the
    [`SemiTrapezoidWidthsAndHeightsBuilder`] (it uses that struct under the
    hood). See the docstring of [`SemiTrapezoidWidthsAndHeightsBuilder`] for
    parameter descriptions.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;
    use stem_slot::prelude::*;

    let slot = SemiTrapezoidSlot::new(
        Length::new::<millimeter>(9.0),
        Length::new::<millimeter>(11.0),
        Length::new::<millimeter>(9.0),
        Length::new::<millimeter>(7.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(16.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.75),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.5),
        true,
    ).expect("valid parameters");
    assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 179.433, epsilon=1e-3);
    ```
     */
    pub fn new(
        bottom_width: Length,
        bottom_side_width: Length,
        top_side_width: Length,
        top_width: Length,
        opening_width: Length,
        bottom_height: Length,
        side_height: Length,
        top_height: Length,
        opening_height: Length,
        bottom_radius: Length,
        bottom_side_radius: Length,
        top_radius: Length,
        top_side_radius: Length,
        opening_radius: Length,
        consider_tooth_tip_leakage: bool,
    ) -> Result<Self, crate::error::Error> {
        SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width,
            bottom_side_width,
            top_side_width,
            top_width,
            opening_width,
            bottom_height,
            side_height,
            top_height,
            opening_height,
            bottom_radius,
            bottom_side_radius,
            top_radius,
            top_side_radius,
            opening_radius,
            consider_tooth_tip_leakage,
        }
        .try_into()
    }

    /// Returns the slot bottom width.
    pub fn bottom_width(&self) -> Length {
        return self.bottom_width;
    }

    /// Returns the width of the winding area at the intersection of the bottom
    /// slope and the slot side.
    pub fn bottom_side_width(&self) -> Length {
        return self.bottom_side_width;
    }

    /// Returns the slot top width.
    pub fn top_width(&self) -> Length {
        return self.top_width;
    }

    /// Returns the vertical height of the slot side.
    pub fn side_height(&self) -> Length {
        return self.side_height;
    }

    /// Returns the angle between the slot sides.
    pub fn slot_angle(&self) -> f64 {
        let delta = 0.5 * (self.bottom_side_width() - self.top_side_width()).get::<meter>();
        return 2.0 * (FRAC_PI_2 - self.side_height().get::<meter>().atan2(delta));
    }

    /// Returns the width of the winding area at the intersection of the top
    /// slope and the slot side.
    pub fn top_side_width(&self) -> Length {
        return self.top_side_width;
    }

    /// Returns the vertical height of the slope at the slot bottom.
    pub fn top_height(&self) -> Length {
        return self.top_height;
    }

    /// Returns the vertical height of the slope at the slot bottom.
    pub fn bottom_height(&self) -> Length {
        return self.bottom_height;
    }

    /// Returns the angle between the bottom slope and the slot bottom. If there
    /// is no bottom slope, this function instead returns the angle between the
    /// slot bottom and the slot sides.
    pub fn bottom_angle(&self) -> f64 {
        if self.bottom_height() == Length::new::<meter>(0.0) {
            return FRAC_PI_2 - 0.5 * self.slot_angle();
        }
        let delta = 0.5 * (self.bottom_side_width() - self.bottom_width()).get::<meter>();
        return PI - self.bottom_height().get::<meter>().atan2(delta);
    }

    /// Returns the angle between the slot side and the bottom slope.
    pub fn bottom_side_angle(&self) -> f64 {
        return calculate_bottom_side_angle(self.bottom_angle(), self.slot_angle());
    }

    /// Returns the angle between the top slope and the slot top. If there
    /// is no top slope, this function instead returns the angle between the
    /// slot top and the slot sides.
    pub fn top_angle(&self) -> f64 {
        if self.top_height() == Length::new::<meter>(0.0) {
            return FRAC_PI_2 + 0.5 * self.slot_angle();
        }
        let delta = 0.5 * (self.top_side_width() - self.top_width()).get::<meter>();
        return PI - self.top_height().get::<meter>().atan2(delta);
    }

    /// Returns the angle between the slot side and the top slope.
    pub fn top_side_angle(&self) -> f64 {
        return calculate_top_side_angle(self.top_angle(), self.slot_angle());
    }

    /// Returns the fillet radius between bottom and bottom slope (if one
    /// exists) or the sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunken to fit the slot geometry.
    pub fn bottom_radius(&self) -> Length {
        return self.bottom_radius;
    }

    /// Returns the fillet radius between bottom slope and sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunken to fit the slot geometry.
    pub fn bottom_side_radius(&self) -> Length {
        return self.bottom_side_radius;
    }

    /// Returns the fillet radius between top and top slope (if one exists) or
    /// the sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunken to fit the slot geometry.
    pub fn top_radius(&self) -> Length {
        return self.top_radius;
    }

    /// Returns the fillet radius between top slope and sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunken to fit the slot geometry.
    pub fn top_side_radius(&self) -> Length {
        return self.top_side_radius;
    }

    /// Returns the fillet radius between slot top and slot opening
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunken to fit the slot geometry.
    pub fn opening_radius(&self) -> Length {
        return self.opening_radius;
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for SemiTrapezoidSlot {
    fn outline(&self) -> Cow<'_, Polysegment> {
        return Cow::Borrowed(&self.outline);
    }

    fn height(&self) -> Length {
        return self.bottom_height + self.side_height + self.top_height + self.opening_height;
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

    fn winding_area(&self) -> Area {
        return self.area()
            - self.opening_height * self.opening_width
            - 2.0 * self.opening_radius * self.opening_radius * (1.0 - PI / 4.0);
    }
}

/// Helper function for calculating the calculate_top_side_angle, not meant for
/// external use.
fn calculate_top_side_angle(top_angle: f64, slot_angle: f64) -> f64 {
    return 3.0 * FRAC_PI_2 - top_angle + 0.5 * slot_angle;
}

/// Helper function for calculating the bottom_side_angle, not meant for
/// external use.
fn calculate_bottom_side_angle(bottom_angle: f64, slot_angle: f64) -> f64 {
    return 3.0 * FRAC_PI_2 - bottom_angle - 0.5 * slot_angle;
}

/**
A builder struct for an [`SemiTrapezoidSlot`] which is functionally equivalent
to [`SemiTrapezoidSlot::new`].

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidWidthsAndHeightsBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidWidthsAndHeightsBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    bottom_side_width: Length::new::<millimeter>(15.603976),
    top_side_width: Length::new::<millimeter>(13.854202),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    bottom_height: Length::new::<millimeter>(2.035763),
    side_height: Length::new::<millimeter>(10.0),
    top_height: Length::new::<millimeter>(1.9642365),
    opening_height: Length::new::<millimeter>(2.0),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWidthsAndHeightsBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot at the corner between bottom slope and slot sides.
    /// Must be positive (`bottom_side_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_width: Length,
    /// Width of the slot at the corner between top slope and slot sides.
    /// Must be positive (`top_side_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidWidthsAndHeightsBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidWidthsAndHeightsBuilder::top_width`] (`top_width >=
    /// opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Height of the bottom slope of the slot. Must not be negative
    /// (`0 m <= bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Side height of the slot. Must not be negative
    /// (`0 m <= side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Height of the top slope of the slot. Must not be negative
    /// (`0 m <= top_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_height: Length,
    /// Height of the slot opening. Must not be negative
    /// (`0 m <= opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidWidthsAndHeightsBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWidthsAndHeightsBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let mut bottom_side_width = builder.bottom_side_width;
        let mut top_side_width = builder.top_side_width;
        let top_width = builder.top_width;
        let opening_width = builder.opening_width;
        let mut bottom_height = builder.bottom_height;
        let mut side_height = builder.side_height;
        let mut top_height = builder.top_height;
        let opening_height = builder.opening_height;
        let mut bottom_radius = builder.bottom_radius;
        let mut top_radius = builder.top_radius;
        let mut bottom_side_radius = builder.bottom_side_radius;
        let mut top_side_radius = builder.top_side_radius;
        let mut opening_radius = builder.opening_radius;

        let zero = Length::new::<meter>(0.0);

        // Set parameters which may be calculated by an algorithm and are close
        // to zero to exactly zero.
        if approx::ulps_eq!(
            bottom_side_width.get::<meter>(),
            0.0,
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            bottom_side_width = zero;
        }
        if approx::ulps_eq!(
            top_side_width.get::<meter>(),
            0.0,
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            top_side_width = zero;
        }
        if approx::ulps_eq!(
            bottom_height.get::<meter>(),
            0.0,
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            bottom_height = zero;
        }
        if approx::ulps_eq!(
            side_height.get::<meter>(),
            0.0,
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            side_height = zero;
        }
        if approx::ulps_eq!(
            top_height.get::<meter>(),
            0.0,
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            top_height = zero;
        }

        compare_variables!(val zero <= bottom_width)?;
        compare_variables!(val zero < bottom_side_width)?;
        compare_variables!(val zero < top_side_width)?;
        compare_variables!(val zero <= opening_width <= top_width)?;
        compare_variables!(val zero <= bottom_height)?;
        compare_variables!(val zero <= side_height)?;
        compare_variables!(val zero <= top_height)?;
        compare_variables!(val zero <= opening_height)?;
        compare_variables!(val zero <= bottom_radius)?;
        compare_variables!(val zero <= bottom_side_radius)?;
        compare_variables!(val zero <= top_radius)?;
        compare_variables!(val zero <= top_side_radius)?;
        compare_variables!(val zero <= opening_radius)?;

        let mut points: Vec<[f64; 2]> = Vec::with_capacity(7);
        let mut radii: Vec<f64> = Vec::with_capacity(7);

        let is_open = opening_width.get::<meter>() > 0.0;
        let height = bottom_height + side_height + top_height + opening_height;

        if is_open {
            points.push([opening_width.get::<meter>() / 2.0, 0.0]);
        }

        points.push([
            opening_width.get::<meter>() / 2.0,
            opening_height.get::<meter>(),
        ]);
        if is_open {
            radii.push(opening_radius.get::<meter>());
        }

        points.push([
            top_width.get::<meter>() / 2.0,
            opening_height.get::<meter>(),
        ]);
        radii.push(top_radius.get::<meter>());

        if approx::ulps_ne!(
            top_side_width.get::<meter>(),
            top_width.get::<meter>(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS,
        ) {
            points.push([
                top_side_width.get::<meter>() / 2.0,
                (opening_height + top_height).get::<meter>(),
            ]);
            radii.push(top_side_radius.get::<meter>());
        }

        if bottom_side_width > bottom_width {
            points.push([
                bottom_side_width.get::<meter>() / 2.0,
                (opening_height + top_height + side_height).get::<meter>(),
            ]);
            radii.push(bottom_side_radius.get::<meter>());
        }

        points.push([bottom_width.get::<meter>() / 2.0, height.get::<meter>()]);
        radii.push(bottom_radius.get::<meter>());

        points.push([0.0, height.get::<meter>()]);

        let mut right_outline_half = Polysegment::from_fillet_chain(&points, &radii);

        // Remove the bottom line segment, it will be recreated when connecting
        // the two halfes
        if bottom_width.get::<meter>() > 0.0 {
            right_outline_half.pop_back();
        }

        let mut left_outline_half = right_outline_half.clone();
        left_outline_half.reverse();
        left_outline_half.line_reflection([0.0, 0.0], [0.0, 1.0]);
        right_outline_half.append(&mut left_outline_half);

        let outline = if is_open {
            // Assert that the outline does not intersect itself
            if let Some(intersection) = right_outline_half
                .intersections_polysegment_par(
                    &right_outline_half,
                    DEFAULT_EPSILON,
                    DEFAULT_MAX_ULPS,
                )
                .find_map_any(|v| Some(v))
            {
                return Err(crate::error::Error::OutlineIntersection {
                    intersection,
                    outline: right_outline_half,
                });
            }
            right_outline_half
        } else {
            let contour = Contour::new(right_outline_half);

            // Assert that the contour does not intersect itself
            if let Some(intersection) = contour
                .intersections_contour_par(&contour, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
                .find_map_any(|v| Some(v))
            {
                return Err(crate::error::Error::OutlineIntersection {
                    intersection,
                    outline: contour.into(),
                });
            }
            contour.into()
        };

        // Check if the radii had to be shrunk to create the fillet chain and
        // update their values accordingly.
        let mut nonzero_radii = Vec::with_capacity(5);
        for r in [
            &mut opening_radius,
            &mut top_radius,
            &mut top_side_radius,
            &mut bottom_side_radius,
            &mut bottom_radius,
        ] {
            if *r > zero {
                nonzero_radii.push(r);
            }
        }
        let mut i = 0;
        for segment in outline.segments() {
            if let Segment::ArcSegment(arc_segment) = segment {
                let current_param = &mut nonzero_radii[i];
                if approx::ulps_ne!(
                    arc_segment.radius(),
                    (*current_param).get::<meter>(),
                    epsilon = DEFAULT_EPSILON,
                    max_ulps = DEFAULT_MAX_ULPS
                ) {
                    **current_param = Length::new::<meter>(arc_segment.radius());
                }
                i += 1;
                if i == nonzero_radii.len() {
                    break;
                }
            }
        }

        return Ok(SemiTrapezoidSlot {
            bottom_width,
            bottom_side_width,
            top_side_width,
            top_width,
            opening_width,
            bottom_height,
            side_height,
            top_height,
            opening_height,
            bottom_radius,
            bottom_side_radius,
            top_radius,
            top_side_radius,
            opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
            outline,
        });
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] using angles and the side height.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Implementation

The [`bottom_height`](SemiTrapezoidSlot::bottom_height) and
[`top_height`](SemiTrapezoidSlot::top_height) are calculated from the angles and
the [`side_height`](SemiTrapezoidSlot::side_height) as shown in the image below:

 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Construction of the semi-trapezoid slot][cad_side_height_angles]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "cad_side_height_angles",
        "docs/img/cad_side_height_angles.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidAnglesSideHeightBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidAnglesSideHeightBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    side_height: Length::new::<millimeter>(10.0),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_angle: (0.8 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidAnglesSideHeightBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidAnglesSideHeightBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesSideHeightBuilder::top_width`] (`top_width >=
    /// opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidAnglesSideHeightBuilder::opening_height`] and
    /// [`SemiTrapezoidAnglesSideHeightBuilder::side_height`] (`height >=
    /// opening_height + side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Side height of the slot. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesSideHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesSideHeightBuilder::opening_height`] (`0 m <=
    /// side_height <= height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Height of the slot opening. Must not be negative and not larger
    /// than [`SemiTrapezoidAnglesSideHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesSideHeightBuilder::side_height`] (`0 m <=
    /// opening_height <= height - side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Angle between the slot bottom and the bottom slope (if one exists) or
    /// the slot sides (if no slope exists).
    pub bottom_angle: BottomAngle,
    /// Angle between the slot top and the top slope (if one exists) or the slot
    /// sides (if no slope exists).
    pub top_angle: TopAngle,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidAnglesSideHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidAnglesSideHeightBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let opening_width = builder.opening_width;
        let top_width = builder.top_width;
        let height = builder.height;
        let opening_height = builder.opening_height;
        let side_height = builder.side_height;
        let bottom_angle = builder.bottom_angle.value();
        let slot_angle = builder.slot_angle;
        let top_angle = builder.top_angle.value();

        let (top_height, bottom_side_width, top_side_width) = {
            let dh = (height - side_height - opening_height).get::<meter>();
            let angle_quotient = (bottom_angle - FRAC_PI_2).tan() / (top_angle - FRAC_PI_2).tan();
            if ulps_eq!(
                dh,
                0.0,
                epsilon = DEFAULT_EPSILON,
                max_ulps = DEFAULT_MAX_ULPS
            ) {
                (Length::new::<meter>(0.0), bottom_width, top_width)
            } else if ulps_eq!(
                angle_quotient,
                -1.0,
                epsilon = DEFAULT_EPSILON,
                max_ulps = DEFAULT_MAX_ULPS
            ) {
                (Length::new::<meter>(0.0), bottom_width, top_width)
            } else {
                let dw = 0.5 * (bottom_width - top_width).get::<meter>();
                let side_height = side_height.get::<meter>();
                let bottom_height = (dh
                    - (dw - side_height * (0.5 * slot_angle).tan())
                        / (top_angle - FRAC_PI_2).tan())
                    / (1.0 + angle_quotient);

                let top_height = dh - bottom_height;
                let bottom_side_width = bottom_width.get::<meter>()
                    + 2.0 * bottom_height * (bottom_angle - FRAC_PI_2).tan();
                let top_side_width =
                    top_width.get::<meter>() + 2.0 * top_height * (top_angle - FRAC_PI_2).tan();
                (
                    Length::new::<meter>(top_height),
                    Length::new::<meter>(bottom_side_width),
                    Length::new::<meter>(top_side_width),
                )
            }
        };
        let bottom_height = height - side_height - top_height - opening_height;

        return SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width,
            bottom_side_width,
            top_side_width,
            top_width,
            opening_width,
            bottom_height,
            side_height,
            top_height,
            opening_height,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] without slopes.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
    bottom_width: Length::new::<millimeter>(16.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 210.34, epsilon=1e-2);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithoutSlopesBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot opening. Must not be negative (`opening_width >= 0
    /// m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Opening height of the slot opening. Must be larger than
    /// [`SemiTrapezoidAnglesSideHeightBuilder::opening_height`]
    /// (`opening_height < height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Opening height of the slot opening. Must not be negative and smaller
    /// than [`SemiTrapezoidAnglesSideHeightBuilder::height`] (`0 m <=
    /// opening_height < height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Radius of the fillet between the slot bottom and slot sides. Must not be
    /// negative (`bottom_radius >= 0 m`). Is shrunken to the maximum possible
    /// value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the slot top and slot sides. Must not be
    /// negative (`top_radius >= 0 m`). Is shrunken to the maximum possible
    /// value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidWithoutSlopesBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithoutSlopesBuilder) -> Result<Self, Self::Error> {
        // Calculate the top width from the bottom width and the slot side height
        let side_height = builder.height - builder.opening_height;
        let top_width = builder.bottom_width - 2.0 * side_height * (builder.slot_angle / 2.0).tan();

        return SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width: builder.bottom_width,
            bottom_side_width: builder.bottom_width,
            top_side_width: top_width,
            top_width,
            opening_width: builder.opening_width,
            bottom_height: Length::new::<meter>(0.0),
            side_height,
            top_height: Length::new::<meter>(0.0),
            opening_height: builder.opening_height,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: Length::new::<meter>(0.0),
            top_radius: builder.top_radius,
            top_side_radius: Length::new::<meter>(0.0),
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] using angles and the top height.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidAnglesTopHeightBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidAnglesTopHeightBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    top_height: Length::new::<millimeter>(1.964236),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_angle: (0.8 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidAnglesTopHeightBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidAnglesTopHeightBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesTopHeightBuilder::top_width`] (`top_width >=
    /// opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidAnglesTopHeightBuilder::opening_height`] and
    /// [`SemiTrapezoidAnglesTopHeightBuilder::top_height`]
    /// (`height >= opening_height + top_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the top slope of the slot. Must be positive and not larger
    /// than [`SemiTrapezoidAnglesTopHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesTopHeightBuilder::opening_height`]
    /// (`0 m < top_height <= height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_height: Length,
    /// Height of the slot opening. Must not be negative and not larger
    /// than [`SemiTrapezoidAnglesTopHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesTopHeightBuilder::top_height`]
    /// (`0 m <= opening_height <= height - top_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Angle between the slot bottom and the bottom slope (if one exists) or
    /// the slot sides (if no slope exists).
    pub bottom_angle: BottomAngle,
    /// Angle between the slot top and the top slope (if one exists) or the slot
    /// sides (if no slope exists).
    pub top_angle: TopAngle,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidAnglesTopHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidAnglesTopHeightBuilder) -> Result<Self, Self::Error> {
        let top_side_width = builder.top_width
            + 2.0 * builder.top_height * (builder.top_angle.value() - FRAC_PI_2).tan();

        let side_height = if ulps_eq!(
            builder.bottom_angle.value(),
            FRAC_PI_2 - builder.slot_angle / 2.0
        ) {
            builder.height - builder.top_height - builder.opening_height
        } else {
            let l1 = Line::from_point_angle(
                [
                    0.5 * builder.bottom_width.get::<meter>(),
                    builder.height.get::<meter>(),
                ],
                builder.bottom_angle.value(),
            );
            let l2 = Line::from_point_angle(
                [
                    top_side_width.get::<meter>() / 2.0,
                    (builder.opening_height + builder.top_height).get::<meter>(),
                ],
                FRAC_PI_2 - builder.slot_angle / 2.0,
            );

            let intersection: [f64; 2] =
                match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                    PrimitiveIntersections::One(p) => p,
                    _ => {
                        return Err(Comparison::new(
                            ComparisonValue::new(
                                calculate_bottom_side_angle(
                                    builder.bottom_angle.value(),
                                    builder.slot_angle,
                                ),
                                Some("bottom_side_angle"),
                            ),
                            ComparisonOperator::Equal,
                            ComparisonValue::new(
                                FRAC_PI_2 - builder.slot_angle / 2.0,
                                Some("side_angle"),
                            ),
                            ComparisonOperator::Equal,
                            None,
                        )
                        .into());
                    }
                };

            Length::new::<meter>(intersection[1]) - builder.top_height - builder.opening_height
        };

        return SemiTrapezoidAnglesSideHeightBuilder {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle: builder.bottom_angle,
            top_angle: builder.top_angle,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] using angles and the bottom height.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidAnglesBottomHeightBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidAnglesBottomHeightBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    bottom_height: Length::new::<millimeter>(2.0357634),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_angle: (0.8 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidAnglesBottomHeightBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::top_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::opening_height`] and
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::bottom_height`]
    /// (`height >= opening_height + bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the bottom slope of the slot. Must not be negative and not
    /// larger than [`SemiTrapezoidAnglesBottomHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::opening_height`]
    /// (`0 m < bottom_height <= height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Height of the slot opening. Must not be negative and not larger
    /// than [`SemiTrapezoidAnglesBottomHeightBuilder::height`] minus
    /// [`SemiTrapezoidAnglesBottomHeightBuilder::bottom_height`]
    /// (`0 m <= opening_height <= height - bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Angle between the slot bottom and the bottom slope (if one exists) or
    /// the slot sides (if no slope exists).
    pub bottom_angle: BottomAngle,
    /// Angle between the slot top and the top slope (if one exists) or the slot
    /// sides (if no slope exists).
    pub top_angle: TopAngle,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidAnglesBottomHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidAnglesBottomHeightBuilder) -> Result<Self, Self::Error> {
        let bottom_side_width = builder.bottom_width
            + 2.0 * builder.bottom_height * (builder.bottom_angle.value() - FRAC_PI_2).tan();

        let side_height = if ulps_eq!(
            (PI - builder.top_angle.value()).rem_euclid(TAU),
            (FRAC_PI_2 - builder.slot_angle / 2.0).rem_euclid(TAU)
        ) {
            builder.height - builder.bottom_height - builder.opening_height
        } else {
            let l1 = Line::from_point_angle(
                [
                    0.5 * builder.top_width.get::<meter>(),
                    builder.opening_height.get::<meter>(),
                ],
                PI - builder.top_angle.value(),
            );
            let l2 = Line::from_point_angle(
                [
                    bottom_side_width.get::<meter>() / 2.0,
                    (builder.height - builder.bottom_height).get::<meter>(),
                ],
                FRAC_PI_2 - builder.slot_angle / 2.0,
            );

            let intersection: [f64; 2] =
                match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                    PrimitiveIntersections::One(p) => p,
                    _ => {
                        return Err(Comparison::new(
                            ComparisonValue::new(
                                calculate_bottom_side_angle(
                                    builder.bottom_angle.value(),
                                    builder.slot_angle,
                                ),
                                Some("bottom_side_angle"),
                            ),
                            ComparisonOperator::Equal,
                            ComparisonValue::new(
                                FRAC_PI_2 - builder.slot_angle / 2.0,
                                Some("side_angle"),
                            ),
                            ComparisonOperator::Equal,
                            None,
                        )
                        .into());
                    }
                };

            builder.height - Length::new::<meter>(intersection[1]) - builder.bottom_height
        };

        return SemiTrapezoidAnglesSideHeightBuilder {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle: builder.bottom_angle,
            top_angle: builder.top_angle,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] using angles and the top side
width.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidAnglesTopSideWidthBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidAnglesTopSideWidthBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    top_side_width: Length::new::<millimeter>(13.854202),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_angle: (0.8 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidAnglesTopSideWidthBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidAnglesTopSideWidthBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesTopSideWidthBuilder::top_width`] (`top_width >=
    /// opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Width of the slot at the corner between top slope and slot sides.
    /// Must be positive (`top_side_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_width: Length,
    /// Total height of the slot. Must not be smaller than
    /// [`SemiTrapezoidAnglesTopSideWidthBuilder::opening_height`]
    /// (`height >= opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot opening. Must not be negative and not smaller than
    /// [`SemiTrapezoidAnglesTopSideWidthBuilder::height`]
    /// (`0 m <= opening_height <= height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Angle between the slot bottom and the bottom slope (if one exists) or
    /// the slot sides (if no slope exists).
    pub bottom_angle: BottomAngle,
    /// Angle between the slot top and the top slope (if one exists) or the slot
    /// sides (if no slope exists).
    pub top_angle: TopAngle,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidAnglesTopSideWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidAnglesTopSideWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.top_side_width - builder.top_width);
        let top_height = delta / (builder.top_angle.value() - FRAC_PI_2).tan();
        return SemiTrapezoidAnglesTopHeightBuilder {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            top_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle: builder.bottom_angle,
            top_angle: builder.top_angle,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] using angles and the bottom side
width.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidAnglesBottomSideWidthBuilder;

let slot: SemiTrapezoidSlot = SemiTrapezoidAnglesBottomSideWidthBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(11.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(16.0),
    bottom_side_width: Length::new::<millimeter>(15.603976),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: PI / 18.0,
    bottom_angle: (0.8 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(1.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(0.5),
    top_side_radius: Length::new::<millimeter>(0.5),
    opening_radius: Length::new::<millimeter>(0.5),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 201.72, epsilon=1e-2);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidAnglesBottomSideWidthBuilder {
    /// Width of the slot bottom. Must not be negative (`bottom_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidAnglesBottomSideWidthBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must not be negative and not larger than
    /// [`SemiTrapezoidAnglesBottomSideWidthBuilder::top_width`] (`top_width >=
    /// opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Width of the slot at the corner between bottom slope and slot sides.
    /// Must be positive (`bottom_side_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_width: Length,
    /// Total height of the slot. Must not be smaller than
    /// [`SemiTrapezoidAnglesBottomSideWidthBuilder::opening_height`]
    /// (`height >= opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot opening. Must not be negative and not smaller than
    /// [`SemiTrapezoidAnglesBottomSideWidthBuilder::height`]
    /// (`0 m <= opening_height <= height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Angle between the slot bottom and the bottom slope (if one exists) or
    /// the slot sides (if no slope exists).
    pub bottom_angle: BottomAngle,
    /// Angle between the slot top and the top slope (if one exists) or the slot
    /// sides (if no slope exists).
    pub top_angle: TopAngle,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidAnglesBottomSideWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidAnglesBottomSideWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.bottom_side_width - builder.bottom_width);
        let bottom_height = delta / (builder.bottom_angle.value() - FRAC_PI_2).tan();
        return SemiTrapezoidAnglesBottomHeightBuilder {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            bottom_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle: builder.bottom_angle,
            top_angle: builder.top_angle,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] in a rotary core with constant
tooth width.

This struct can be (fallibly) converted into a [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Implementation

The `top_side_width` and `bottom_side_width` parameters are calculated from the
[`SemiTrapezoidFromToothWidthRotBuilder::tooth_width`],
[`SemiTrapezoidFromToothWidthRotBuilder::air_gap_radius`] and
[`SemiTrapezoidFromToothWidthRotBuilder::slots`] (which is used to derive the
slot angle, see the field docstring). Once those are known, the
[`SemiTrapezoidWidthsAndHeightsBuilder`] can be used to create the slot.

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidFromToothWidthRotBuilder;

let builder = SemiTrapezoidFromToothWidthRotBuilder {
    tooth_width: Length::new::<millimeter>(6.0),
    air_gap_radius: Length::new::<millimeter>(50.0),
    yoke_radius: Length::new::<millimeter>(80.0),
    slots: 36,
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    side_height: Length::new::<millimeter>(17.0),
    bottom_height: Length::new::<millimeter>(0.0),
    top_height: Length::new::<millimeter>(0.0),
    opening_height: Length::new::<millimeter>(0.75),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 73.2420, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidFromToothWidthRotBuilder {
    /// Constant width of the teeth between the slots. Is used to determine the
    /// slot widths. Must be positive (`tooth_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub tooth_width: Length,
    /// Air gap radius of the magnetic core. If smaller than
    /// [`SemiTrapezoidFromToothWidthRotBuilder::yoke_radius`], the slots are
    /// created for an outer core, otherwise for an inner core. Must be positive
    /// (`air_gap_radius > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub air_gap_radius: Length,
    /// Yoke radius of the magnetic core. If smaller than
    /// [`SemiTrapezoidFromToothWidthRotBuilder::air_gap_radius`], the slots are
    /// created for an inner core, otherwise for an outer core. Must be positive
    /// (`air_gap_radius > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub yoke_radius: Length,
    /// Number of slots. The slot angle is calculated as `2 * PI / slots`.
    pub slots: u16,
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must not be smaller than
    /// [`SemiTrapezoidFromToothWidthRotBuilder::opening_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive, but not larger than
    /// [`SemiTrapezoidFromToothWidthRotBuilder::top_width`]
    /// (`top_width >= opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Height of the bottom slope of the slot. Must not be negative
    /// (`0 m <= bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Side height of the slot. Must not be negative
    /// (`0 m <= side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Height of the top slope of the slot. Must not be negative
    /// (`0 m <= top_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_height: Length,
    /// Height of the slot opening. Must not be negative
    /// (`0 m <= opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunken to the
    /// maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::bottom_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_side_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidFromToothWidthRotBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidFromToothWidthRotBuilder) -> Result<Self, Self::Error> {
        let [bottom_side_width, top_side_width] = slot_side_bottom_and_top_width_from_rot_core(
            builder.tooth_width,
            builder.air_gap_radius,
            builder.yoke_radius,
            builder.slots,
            builder.side_height,
            builder.opening_width,
            builder.opening_height,
        );

        let bottom_width = if builder.bottom_height == Length::new::<millimeter>(0.0) {
            bottom_side_width
        } else {
            builder.bottom_width
        };

        let top_width = if builder.top_height == Length::new::<millimeter>(0.0) {
            top_side_width
        } else {
            builder.top_width
        };

        return SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width,
            bottom_side_width,
            top_side_width,
            top_width,
            opening_width: builder.opening_width,
            bottom_height: builder.bottom_height,
            side_height: builder.side_height,
            top_height: builder.top_height,
            opening_height: builder.opening_height,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: builder.bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius: builder.top_side_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] in a rotary core with constant
tooth width and without slopes.

This struct can be (fallibly) converted into a[`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges.

Even with all parameters being inside the value ranges, some parameter
combinations might still result in intersecting slot outlines, in which case the
conversion attempt will return an
[`Error::OutlineIntersection`](crate::error::Error::OutlineIntersection).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi trapezoid slot definitions][cad_semi_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_semi_trapezoid", "docs/img/cad_semi_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Implementation

The `top_width` and `bottom_width` parameters can be calculated from the
[`SemiTrapezoidFromToothWidthRotBuilder::tooth_width`],
[`SemiTrapezoidFromToothWidthRotBuilder::air_gap_radius`] and
[`SemiTrapezoidFromToothWidthRotBuilder::slots`], since the slot is known to
have no slopes. With those parameters, the
[`SemiTrapezoidWidthsAndHeightsBuilder`] can be used to create the slot.

# Examples

```
use approx::assert_abs_diff_eq;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::semi_trapezoid::SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder;

let builder = SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder {
    tooth_width: Length::new::<millimeter>(6.0),
    air_gap_radius: Length::new::<millimeter>(50.0),
    yoke_radius: Length::new::<millimeter>(80.0),
    slots: 36,
    opening_width: Length::new::<millimeter>(2.0),
    side_height: Length::new::<millimeter>(17.0),
    opening_height: Length::new::<millimeter>(0.75),
    bottom_radius: Length::new::<millimeter>(2.0),
    top_radius: Length::new::<millimeter>(2.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 73.2420, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder {
    /// Constant width of the teeth between the slots. Is used to determine the
    /// slot widths. Must be positive (`tooth_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub tooth_width: Length,
    /// Air gap radius of the magnetic core. If smaller than
    /// [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder::yoke_radius`], the
    /// slots are created for an outer core, otherwise for an inner core. Must
    /// be positive (`air_gap_radius > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub air_gap_radius: Length,
    /// Yoke radius of the magnetic core. If smaller than
    /// [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder::air_gap_radius`],
    /// the slots are created for an inner core, otherwise for an outer core.
    /// Must be positive (`air_gap_radius > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub yoke_radius: Length,
    /// Number of slots. The slot angle is calculated as `2 * PI / slots`.
    pub slots: u16,
    /// Width of the slot opening. Must not be positive
    /// (`opening_width >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Side height of the slot. Must not be negative
    /// (`0 m <= side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Height of the slot opening. Must not be negative
    /// (`0 m <= opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`). Is shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::bottom_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the slot top and top slope (if one exists)
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). is
    /// shrunken to the maximum possible value if required by the slot
    /// geometry, see [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the slot top and the slot opening. Must not
    /// be negative (`opening_radius >= 0 m`). Is shrunken to the maximum
    /// possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::opening_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    /// implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(
        builder: SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder,
    ) -> Result<Self, Self::Error> {
        let [bottom_width, top_width] = slot_side_bottom_and_top_width_from_rot_core(
            builder.tooth_width,
            builder.air_gap_radius,
            builder.yoke_radius,
            builder.slots,
            builder.side_height,
            builder.opening_width,
            builder.opening_height,
        );

        return SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width: bottom_width,
            bottom_side_width: bottom_width,
            top_side_width: top_width,
            top_width: top_width,
            opening_width: builder.opening_width,
            bottom_height: Length::new::<meter>(0.0),
            side_height: builder.side_height,
            top_height: Length::new::<meter>(0.0),
            opening_height: builder.opening_height,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius: Length::new::<meter>(0.0),
            top_radius: builder.top_radius,
            top_side_radius: Length::new::<meter>(0.0),
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SemiTrapezoidSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum SlotEnum {
            SemiTrapezoidWidthsAndHeightsBuilder(SemiTrapezoidWidthsAndHeightsBuilder),
            SemiTrapezoidAnglesSideHeightBuilder(SemiTrapezoidAnglesSideHeightBuilder),
            SemiTrapezoidWithoutSlopesBuilder(SemiTrapezoidWithoutSlopesBuilder),
            SemiTrapezoidAnglesTopHeightBuilder(SemiTrapezoidAnglesTopHeightBuilder),
            SemiTrapezoidAnglesBottomHeightBuilder(SemiTrapezoidAnglesBottomHeightBuilder),
            SemiTrapezoidAnglesTopSideWidthBuilder(SemiTrapezoidAnglesTopSideWidthBuilder),
            SemiTrapezoidAnglesBottomSideWidthBuilder(SemiTrapezoidAnglesBottomSideWidthBuilder),
            SemiTrapezoidFromToothWidthRotBuilder(SemiTrapezoidFromToothWidthRotBuilder),
            SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder(
                SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder,
            ),
        }
        let se = SlotEnum::deserialize(deserializer)?;
        match se {
            SlotEnum::SemiTrapezoidWidthsAndHeightsBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidAnglesSideHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithoutSlopesBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidAnglesTopHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidAnglesBottomHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidAnglesTopSideWidthBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidAnglesBottomSideWidthBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidFromToothWidthRotBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}
