/*!
This module defines a [`SemiTrapezoidSlot`] - a trapezoid slot which is
semi-opened or even closed towards the air gap - as well as a couple of
"builder" structs which can be used to create a [`SemiTrapezoidSlot`]. See the
struct documentation for more.

Additionally, it defines the [`BottomAngle`] and
[`TopAngle`] structs for calculating slot angles from width and
height parameters. These are used as parameters for some of the builder structs.
 */

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
can be fallibly converted to an [`OpenTrapezoidSlot`] via their [`TryFrom`]
implementations:
- [`SemiTrapezoidBuilder`] (builder version of [`new`](SemiTrapezoidSlot::new))
- [`SemiTrapezoidWithoutSlopesBuilder`]
- [`SemiTrapezoidWithTopHeightBuilder`]
- [`SemiTrapezoidWithBottomHeightBuilder`]
- [`SemiTrapezoidWithTopSideWidthBuilder`]
- [`SemiTrapezoidWithBottomSideWidthBuilder`]
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
docstring of the respective builder struct.

Using structs instead of constructor functions makes it less likely to confuse
arguments, since the parameter name needs to be specified explicitly. For
convenience, there exists a constructor function [`SemiTrapezoidSlot::new`]
which internally creates an [`SemiTrapezoidBuilder`] and then converts it.

# Serialization and deserialization

This struct can be directly deserialized from any of its "builder" structs (no
need for a tag). Its serialized form is that of the [`SemiTrapezoidBuilder`]
struct.

```
use approx;
use stem_slot::prelude::*;
use serde_yaml;

// Parameters of a SemiTrapezoidBuilder
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
    top_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    side_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_height: Length,
    slot_angle: f64,
    bottom_angle: f64,
    top_angle: f64,
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

    This is the function equivalent for the [`SemiTrapezoidBuilder`] (and in
    fact creates a builder under the hood which is then converted to the final
    slot type). See the docstring of the builder struct for parameter
    descriptions.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;
    use stem_slot::prelude::*;

    let slot = SemiTrapezoidSlot::new(
        Length::new::<millimeter>(9.0),
        Length::new::<millimeter>(7.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(0.75),
        Length::new::<millimeter>(14.0),
        PI / 18.0,
        PI * 0.7,
        PI * 0.7,
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.5),
        true,
    ).expect("valid parameters");
    assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 171.635, epsilon=1e-3);
    ```
     */
    pub fn new<B: Into<BottomAngle>, T: Into<TopAngle>>(
        bottom_width: Length,
        top_width: Length,
        opening_width: Length,
        height: Length,
        opening_height: Length,
        side_height: Length,
        slot_angle: f64,
        bottom_angle: B,
        top_angle: T,
        bottom_radius: Length,
        bottom_side_radius: Length,
        top_radius: Length,
        top_side_radius: Length,
        opening_radius: Length,
        consider_tooth_tip_leakage: bool,
    ) -> Result<Self, crate::error::Error> {
        SemiTrapezoidBuilder {
            bottom_width,
            opening_width,
            height,
            side_height,
            opening_height,
            slot_angle,
            bottom_radius,
            bottom_side_radius,
            consider_tooth_tip_leakage,
            top_width,
            bottom_angle: bottom_angle.into(),
            top_angle: top_angle.into(),
            top_radius,
            top_side_radius,
            opening_radius,
        }
        .try_into()
    }

    /// Returns the slot bottom width.
    pub fn bottom_width(&self) -> Length {
        let params = self.calculate_params();
        return params.bottom_side_width;
    }

    /// Returns the width of the winding area at the intersection of the bottom
    /// slope and the slot side.
    pub fn bottom_side_width(&self) -> Length {
        return self.calculate_params().bottom_side_width;
    }

    /// Returns the slot top width.
    pub fn top_width(&self) -> Length {
        return self.top_width;
    }

    /// Returns the vertical height of the slot side.
    pub fn side_height(&self) -> Length {
        return self.side_height;
    }

    /// Returns the width of the winding area at the intersection of the top
    /// slope and the slot side.
    pub fn top_side_width(&self) -> Length {
        return self.calculate_params().top_side_width;
    }

    /// Returns the vertical height of the slope at the slot bottom.
    pub fn top_height(&self) -> Length {
        return self.calculate_params().top_height;
    }

    /// Returns the vertical height of the slope at the slot bottom.
    pub fn bottom_height(&self) -> Length {
        return self.height - self.side_height - self.opening_height - self.top_height();
    }

    /// Returns the angle between the bottom slope and the slot bottom.
    pub fn bottom_angle(&self) -> f64 {
        return self.bottom_angle;
    }

    /// Returns the angle between the slot side and the bottom slope.
    pub fn bottom_side_angle(&self) -> f64 {
        return self.calculate_params().bottom_side_angle;
    }

    /// Returns the angle between the top slope and the slot top.
    pub fn top_angle(&self) -> f64 {
        return self.top_angle;
    }

    /// Returns the angle between the slot side and the top slope.
    pub fn top_side_angle(&self) -> f64 {
        return self.calculate_params().top_side_angle;
    }

    /// Returns the fillet radius between bottom and bottom slope (if one
    /// exists) or the sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunk to fit the slot geometry.
    pub fn bottom_radius(&self) -> Length {
        return self.bottom_radius;
    }

    /// Returns the fillet radius between bottom slope and sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunk to fit the slot geometry.
    pub fn bottom_side_radius(&self) -> Length {
        return self.bottom_side_radius;
    }

    /// Returns the fillet radius between top and top slope (if one exists) or
    /// the sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunk to fit the slot geometry.
    pub fn top_radius(&self) -> Length {
        return self.top_radius;
    }

    /// Returns the fillet radius between top slope and sides.
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunk to fit the slot geometry.
    pub fn top_side_radius(&self) -> Length {
        return self.top_side_radius;
    }

    /// Returns the fillet radius between slot top and slot opening
    ///
    /// This value can be smaller than the provided radius, because the radius
    /// is shrunk to fit the slot geometry.
    pub fn opening_radius(&self) -> Length {
        return self.opening_radius;
    }

    /// Calculates some parameters of `self`.
    fn calculate_params(&self) -> CalculatedParams {
        let bottom_side_angle = calculate_bottom_side_angle(self.bottom_angle, self.slot_angle);
        let top_side_angle = calculate_top_side_angle(self.top_angle, self.slot_angle);
        let delta_side_width = self.side_height * (self.slot_angle / 2.0).tan();

        /*
        Now the slope points must fulfill two conditions:
        Δb_side = (side_bottom_width - side_top_width)/2 (1)
        side_height + top_height + bottom_height = height (2)

        To solve this, the following equations are used:
        bottom_height = Δ_bottom*tand(α) (3)
        top_height = Δ_top*tand(β) (4)
        side_bottom_width = bottom_width + 2*Δ_bottom (5)
        side_top_width = top_width + 2*Δ_top (6)

        Solving by substitution:
        (5) and (6) in (1)
        Δb_side = (bottom_width + 2*Δ_bottom - top_width - 2*Δ_top)/2 (7)
        (3) and (4) in (7)
        Δb_side = (bottom_width + 2*bottom_height/tand(α) - top_width - 2*top_height/tand(β))/2 (8)

        Solve (2) for top_height
        top_height = height - side_height - bottom_height - opening_height (9)
        (9) in (8)
        Δb_side = (bottom_width + 2*bottom_height/tand(α) - top_width - 2*(height - side_height - bottom_height - opening_height)/tand(β))/2 (10)

        # Now solve (10) for bottom_height
        bottom_height = (2*Δb_side - bottom_width + top_width + 2*(height - side_height - opening_height)/tand(β))/(2/tand(α) + 2/tand(β))
        */
        let bottom_height: Length;
        let bottom_side_width: Length;
        if approx::abs_diff_eq!(bottom_side_angle, PI, epsilon = 1e-15) {
            bottom_height = Length::new::<meter>(0.0);
            bottom_side_width = self.bottom_width;
        } else {
            bottom_height = (2.0 * delta_side_width - self.bottom_width
                + self.top_width
                + 2.0 * (self.height - self.side_height - self.opening_height)
                    / self.top_angle.tan())
                / (2.0 / bottom_side_angle.tan() + 2.0 / self.top_angle.tan());
            bottom_side_width =
                self.bottom_width + 2.0 * bottom_height * (self.bottom_angle - FRAC_PI_2).tan();
        }

        let top_height = self.height - self.side_height - bottom_height - self.opening_height;
        let top_side_width = if approx::abs_diff_eq!(top_side_angle, PI, epsilon = 1e-15) {
            self.top_width
        } else {
            self.top_width + 2.0 * top_height * (self.top_angle - FRAC_PI_2).tan()
        };

        return CalculatedParams {
            top_height,
            bottom_side_width,
            top_side_width,
            bottom_side_angle,
            top_side_angle,
        };
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for SemiTrapezoidSlot {
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

    fn winding_area(&self) -> Area {
        return self.area()
            - self.opening_height * self.opening_width
            - 2.0 * self.opening_radius * self.opening_radius * (1.0 - PI / 4.0);
    }
}

/// Helper struct for the calculation of dependent properties, not meant for
/// external use.
struct CalculatedParams {
    top_height: Length,
    bottom_side_width: Length,
    top_side_width: Length,
    bottom_side_angle: f64,
    top_side_angle: f64,
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

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidBuilder;

let builder = SemiTrapezoidBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    side_height: Length::new::<millimeter>(17.0),
    opening_height: Length::new::<millimeter>(0.75),
    bottom_angle: (0.7 * PI).into(),
    top_angle: (0.7 * PI).into(),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(0.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(0.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 137.542, epsilon=1e-3);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidBuilder::opening_height`] and
    /// [`SemiTrapezoidBuilder::side_height`] (`height >= opening_height +
    /// side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Side height of the slot. Must be positive and not larger than
    /// [`SemiTrapezoidBuilder::height`] minus
    /// [`SemiTrapezoidBuilder::opening_height`] (`0 m < side_height <=
    /// height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Height of the slot opening. Must not be negative and not larger
    /// than [`SemiTrapezoidBuilder::height`] minus
    /// [`SemiTrapezoidBuilder::side_height`] (`0 m <= opening_height <=
    /// height - side_height`).
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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

impl TryFrom<SemiTrapezoidBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let opening_width = builder.opening_width;
        let top_width = builder.top_width;
        let height = builder.height;
        let mut bottom_radius = builder.bottom_radius;
        let mut top_radius = builder.top_radius;
        let mut bottom_side_radius = builder.bottom_side_radius;
        let mut top_side_radius = builder.top_side_radius;
        let mut opening_radius = builder.opening_radius;
        let opening_height = builder.opening_height;
        let side_height = builder.side_height;
        let bottom_angle = builder.bottom_angle.value();
        let slot_angle = builder.slot_angle;
        let top_angle = builder.top_angle.value();

        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < bottom_width)?;
        compare_variables!(val zero <= opening_width)?;
        compare_variables!(val zero <= top_width)?;
        compare_variables!(val zero < height)?;
        compare_variables!(val zero <= opening_height)?;
        compare_variables!(val zero <= bottom_radius)?;
        compare_variables!(val zero <= bottom_side_radius)?;
        compare_variables!(val zero <= top_radius)?;
        compare_variables!(val zero <= top_side_radius)?;
        compare_variables!(val zero <= opening_radius)?;
        compare_variables!(opening_height < height)?;

        // Points 0 - 6 as defined in [20201109_BerechnungNut.pdf]
        let mut points: Vec<[f64; 2]> = Vec::with_capacity(7);
        let mut radii: Vec<f64> = Vec::with_capacity(7);

        let mut this = Self {
            bottom_width,
            top_width,
            opening_width,
            height,
            side_height,
            opening_height,
            slot_angle,
            bottom_angle,
            top_angle,
            bottom_radius,
            bottom_side_radius,
            top_radius,
            top_side_radius,
            opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
            outline: Polysegment::new(),
        };

        let params = this.calculate_params();

        let is_open = builder.opening_width.get::<meter>() > 0.0;

        // Vertex 1
        if is_open {
            points.push([builder.opening_width.get::<meter>() / 2.0, 0.0]);
        }

        // Vertex 2
        points.push([
            builder.opening_width.get::<meter>() / 2.0,
            builder.opening_height.get::<meter>(),
        ]);
        if is_open {
            radii.push(builder.opening_radius.get::<meter>());
        }

        // Vertex 3
        points.push([
            builder.top_width.get::<meter>() / 2.0,
            builder.opening_height.get::<meter>(),
        ]);
        radii.push(builder.top_radius.get::<meter>());

        // Vertex 4
        if approx::ulps_ne!(
            params.top_side_width.get::<meter>(),
            builder.top_width.get::<meter>(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS,
        ) {
            points.push([
                params.top_side_width.get::<meter>() / 2.0,
                (builder.opening_height + params.top_height).get::<meter>(),
            ]);
            radii.push(builder.top_side_radius.get::<meter>());
        }

        // Vertex 5
        if params.bottom_side_width > builder.bottom_width {
            points.push([
                params.bottom_side_width.get::<meter>() / 2.0,
                (builder.opening_height + params.top_height + builder.side_height).get::<meter>(),
            ]);
            radii.push(builder.bottom_side_radius.get::<meter>());
        }

        // Vertex 6
        points.push([
            builder.bottom_width.get::<meter>() / 2.0,
            builder.height.get::<meter>(),
        ]);
        radii.push(builder.bottom_radius.get::<meter>());

        // Mirror the points along the y-axis
        let n_points_half = points.len();
        for i in 0..n_points_half {
            let i_rev = n_points_half - i - 1;
            let pt = points[i_rev];
            points.push([-pt[0], pt[1]]);
        }

        let n_radii_half = radii.len();
        for i in 0..n_radii_half {
            let i_rev = n_radii_half - i - 1;
            radii.push(radii[i_rev]);
        }

        let outline = if is_open {
            let outline = Polysegment::from_fillet_chain(&points, &radii);

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
            outline
        } else {
            let contour = Contour::new(Polysegment::from_fillet_chain(&points, &radii));

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

        this.outline = outline;
        return Ok(this);
    }
}

/**
A builder struct for an [`SemiTrapezoidSlot`] without slopes.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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

let builder = SemiTrapezoidWithoutSlopesBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    top_radius: Length::new::<millimeter>(2.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 125.793, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithoutSlopesBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Opening height of the slot opening. Must be larger than
    /// [`SemiTrapezoidBuilder::opening_height`] (`opening_height < height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Opening height of the slot opening. Must not be negative and smaller
    /// than [`SemiTrapezoidBuilder::height`] (`0 m <= opening_height <
    /// height`).
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
    /// negative (`bottom_radius >= 0 m`). Is shrunk to the maximum possible
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
    /// negative (`top_radius >= 0 m`). Is shrunk to the maximum possible
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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
        let top_width = builder.bottom_width
            - 2.0 * (builder.height - builder.opening_height) * (builder.slot_angle / 2.0).tan();
        let side_height = builder.height - builder.opening_height;

        let top_angle = TopAngle::new_no_slope(builder.slot_angle);
        let bottom_angle = BottomAngle::new_no_slope(builder.slot_angle);

        return SemiTrapezoidBuilder {
            bottom_width: builder.bottom_width,
            top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle,
            top_angle,
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
A builder struct for an [`SemiTrapezoidSlot`] where the top height is specified.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidWithTopHeightBuilder;

let builder = SemiTrapezoidWithTopHeightBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    top_height: Length::new::<millimeter>(2.0),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_angle: (0.7 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 135.655, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithTopHeightBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidWithTopHeightBuilder::opening_height`] and
    /// [`SemiTrapezoidWithTopHeightBuilder::top_height`]
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
    /// than [`SemiTrapezoidWithTopHeightBuilder::height`] minus
    /// [`SemiTrapezoidWithTopHeightBuilder::opening_height`]
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
    /// than [`SemiTrapezoidWithTopHeightBuilder::height`] minus
    /// [`SemiTrapezoidWithTopHeightBuilder::top_height`]
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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

impl TryFrom<SemiTrapezoidWithTopHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithTopHeightBuilder) -> Result<Self, Self::Error> {
        let gamma = calculate_top_side_angle(builder.top_angle.value(), builder.slot_angle);
        let top_side_width = builder.top_width + 2.0 * builder.top_height / gamma.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let bottom_side_angle =
            calculate_bottom_side_angle(builder.bottom_angle.value(), builder.slot_angle);
        let top_side_angle = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.bottom_width.get::<meter>(),
                builder.height.get::<meter>(),
            ],
            bottom_side_angle,
        );
        let l2 = Line::from_point_angle(
            [
                top_side_width.get::<meter>() / 2.0,
                (builder.opening_height + builder.top_height).get::<meter>(),
            ],
            top_side_angle,
        );

        let intersection: [f64; 2] =
            match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                PrimitiveIntersections::One(p) => p,
                _ => {
                    return Err(Comparison::new(
                        ComparisonValue::new(bottom_side_angle, Some("angle of slot bottom slope")),
                        ComparisonOperator::Equal,
                        ComparisonValue::new(top_side_angle, Some("perpendicular to slot side")),
                        ComparisonOperator::Equal,
                        None,
                    )
                    .into());
                }
            };

        let bottom_height = builder.height - Length::new::<meter>(intersection[1]);
        let side_height =
            builder.height - builder.top_height - bottom_height - builder.opening_height;

        return SemiTrapezoidBuilder {
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
A builder struct for an [`SemiTrapezoidSlot`] where the bottom height is
specified.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidWithBottomHeightBuilder;

let builder = SemiTrapezoidWithBottomHeightBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    bottom_height: Length::new::<millimeter>(2.0),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_angle: (0.7 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 176.167, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithBottomHeightBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must not be smaller than the sum of
    /// [`SemiTrapezoidWithBottomHeightBuilder::opening_height`] and
    /// [`SemiTrapezoidWithBottomHeightBuilder::bottom_height`]
    /// (`height >= opening_height + bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the bottom slope of the slot. Must be positive and not larger
    /// than [`SemiTrapezoidWithBottomHeightBuilder::height`] minus
    /// [`SemiTrapezoidWithBottomHeightBuilder::opening_height`]
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
    /// than [`SemiTrapezoidWithBottomHeightBuilder::height`] minus
    /// [`SemiTrapezoidWithBottomHeightBuilder::bottom_height`]
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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

impl TryFrom<SemiTrapezoidWithBottomHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithBottomHeightBuilder) -> Result<Self, Self::Error> {
        let bottom_side_angle =
            calculate_bottom_side_angle(builder.bottom_angle.value(), builder.slot_angle);
        let bottom_side_width =
            builder.bottom_width + 2.0 * builder.bottom_height / bottom_side_angle.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 4 (side to slope_top)
        let top_side_angle =
            calculate_top_side_angle(builder.top_angle.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ],
            top_side_angle,
        );
        let l2 = Line::from_point_angle(
            [
                bottom_side_width.get::<meter>() / 2.0,
                (builder.height - builder.bottom_height).get::<meter>(),
            ],
            gamma,
        );

        let intersection: [f64; 2] =
            match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                PrimitiveIntersections::One(p) => p,
                _ => {
                    return Err(Comparison::new(
                        ComparisonValue::new(bottom_side_angle, Some("angle of slot bottom slope")),
                        ComparisonOperator::Equal,
                        ComparisonValue::new(top_side_angle, Some("perpendicular to slot side")),
                        ComparisonOperator::Equal,
                        None,
                    )
                    .into());
                }
            };

        let top_height = Length::new::<meter>(intersection[1]) - builder.opening_height;
        let side_height =
            builder.height - top_height - builder.bottom_height - builder.opening_height;

        return SemiTrapezoidBuilder {
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
A builder struct for an [`SemiTrapezoidSlot`] where the top side width is
specified.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidWithTopSideWidthBuilder;

let builder = SemiTrapezoidWithTopSideWidthBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    top_side_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_angle: (0.7 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 137.934, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithTopSideWidthBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Width of the slot at the corner between top slope and slot sides. Must
    /// be larger than  [`SemiTrapezoidWithTopSideWidthBuilder::top_width`]
    /// (`top_side_width > top_width`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_side_width: Length,
    /// Total height of the slot. Must be larger than
    /// [`SemiTrapezoidWithTopSideWidthBuilder::opening_height`]
    /// (`height > opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot opening. Must not be negative and smaller than
    /// [`SemiTrapezoidWithTopSideWidthBuilder::height`]
    /// (`0 m <= opening_height < height`).
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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

impl TryFrom<SemiTrapezoidWithTopSideWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithTopSideWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.top_side_width - builder.top_width);
        let top_side_angle =
            calculate_top_side_angle(builder.top_angle.value(), builder.slot_angle);
        let top_height = delta * (top_side_angle - FRAC_PI_2 - 0.5 * builder.slot_angle).tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let bottom_side_angle =
            -calculate_bottom_side_angle(builder.bottom_angle.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.bottom_width.get::<meter>(),
                builder.height.get::<meter>(),
            ],
            bottom_side_angle,
        );
        let l2 = Line::from_point_angle(
            [
                0.5 * builder.top_side_width.get::<meter>(),
                (builder.opening_height + top_height).get::<meter>(),
            ],
            gamma,
        );

        let intersection: [f64; 2] =
            match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                PrimitiveIntersections::One(p) => p,
                PrimitiveIntersections::Zero => {
                    let l1 = Line::from_point_angle(
                        [
                            builder.bottom_width.get::<meter>() / 2.0,
                            builder.height.get::<meter>(),
                        ],
                        bottom_side_angle,
                    );
                    let l2 = Line::from_point_angle(
                        [
                            builder.top_side_width.get::<meter>() / 2.0,
                            (builder.opening_height + top_height).get::<meter>(),
                        ],
                        gamma,
                    );

                    match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                        PrimitiveIntersections::One(p) => p,
                        _ => {
                            return Err(Comparison::new(
                                ComparisonValue::new(
                                    bottom_side_angle,
                                    Some("angle of slot bottom slope"),
                                ),
                                ComparisonOperator::Equal,
                                ComparisonValue::new(
                                    top_side_angle,
                                    Some("perpendicular to slot side"),
                                ),
                                ComparisonOperator::Equal,
                                None,
                            )
                            .into());
                        }
                    }
                }
                _ => {
                    return Err(Comparison::new(
                        ComparisonValue::new(bottom_side_angle, Some("angle of slot bottom slope")),
                        ComparisonOperator::Equal,
                        ComparisonValue::new(top_side_angle, Some("perpendicular to slot side")),
                        ComparisonOperator::Equal,
                        None,
                    )
                    .into());
                }
            };

        let bottom_height = builder.height - Length::new::<meter>(intersection[1]);
        let side_height = builder.height - top_height - bottom_height - builder.opening_height;

        return SemiTrapezoidBuilder {
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
A builder struct for an [`SemiTrapezoidSlot`] where the top side width is
specified.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidWithBottomSideWidthBuilder;

let builder = SemiTrapezoidWithBottomSideWidthBuilder {
    bottom_width: Length::new::<millimeter>(8.0),
    bottom_side_width: Length::new::<millimeter>(10.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_angle: (0.7 * PI).into(),
    top_angle: (0.7 * PI).into(),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 127.623, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithBottomSideWidthBuilder {
    /// Width of the slot bottom. Must be positive (`bottom_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_width: Length,
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Width of the slot at the corner between bottom slope and slot sides.
    /// Must be larger than
    /// [`SemiTrapezoidWithBottomSideWidthBuilder::bottom_width`]
    /// (`bottom_side_width > bottom_width`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_width: Length,
    /// Total height of the slot. Must be larger than
    /// [`SemiTrapezoidWithBottomSideWidthBuilder::opening_height`]
    /// (`height > opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot opening. Must not be negative and smaller than
    /// [`SemiTrapezoidWithBottomSideWidthBuilder::height`]
    /// (`0 m <= opening_height < height`).
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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

impl TryFrom<SemiTrapezoidWithBottomSideWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithBottomSideWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.bottom_side_width - builder.bottom_width);
        let bottom_side_angle =
            calculate_bottom_side_angle(builder.bottom_angle.value(), builder.slot_angle);
        let bottom_height = delta * bottom_side_angle.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let top_side_angle =
            calculate_top_side_angle(builder.top_angle.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.bottom_side_width.get::<meter>(),
                (builder.height - bottom_height).get::<meter>(),
            ],
            gamma,
        );
        let l2 = Line::from_point_angle(
            [
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ],
            top_side_angle,
        );

        let intersection: [f64; 2] =
            match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                PrimitiveIntersections::One(p) => p,
                _ => {
                    return Err(Comparison::new(
                        ComparisonValue::new(bottom_side_angle, Some("angle of slot bottom slope")),
                        ComparisonOperator::Equal,
                        ComparisonValue::new(top_side_angle, Some("perpendicular to slot side")),
                        ComparisonOperator::Equal,
                        None,
                    )
                    .into());
                }
            };

        let top_height = Length::new::<meter>(intersection[1]) - builder.opening_height;
        let side_height = builder.height - top_height - bottom_height - builder.opening_height;

        return SemiTrapezoidBuilder {
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
A builder struct for an [`SemiTrapezoidSlot`] in a rotary core with constant
tooth width.

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidFromToothWidthRotBuilder;

let builder = SemiTrapezoidFromToothWidthRotBuilder {
    tooth_width: Length::new::<millimeter>(6.0),
    air_gap_radius: Length::new::<millimeter>(50.0),
    yoke_radius: Length::new::<millimeter>(80.0),
    slots: 36,
    bottom_width: Length::new::<millimeter>(9.0),
    top_width: Length::new::<millimeter>(7.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    bottom_radius: Length::new::<millimeter>(2.0),
    bottom_side_radius: Length::new::<millimeter>(1.0),
    top_radius: Length::new::<millimeter>(2.0),
    top_side_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 137.934, epsilon=1e-3);
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
    /// Width of the slot top. Must be positive (`top_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_width: Length,
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must be larger than or equal to the sum of
    /// [`SemiTrapezoidFromToothWidthRotBuilder::top_height`],
    /// [`SemiTrapezoidFromToothWidthRotBuilder::bottom_height`] and
    /// [`SemiTrapezoidFromToothWidthRotBuilder::opening_height`]
    /// (`height >= top_height + bottom_height + opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the bottom slope. Must not be negative and not larger
    /// than [`SemiTrapezoidFromToothWidthRotBuilder::height`] minus
    /// [`SemiTrapezoidFromToothWidthRotBuilder::top_height`] and
    /// [`SemiTrapezoidFromToothWidthRotBuilder::opening_height`]
    /// (`0 m <= bottom_height <= height - opening_height - top_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Height of the top slope. Must not be negative and not larger
    /// than [`SemiTrapezoidFromToothWidthRotBuilder::height`] minus
    /// [`SemiTrapezoidFromToothWidthRotBuilder::bottom_height`] and
    /// [`SemiTrapezoidFromToothWidthRotBuilder::opening_height`]
    /// (`0 m <= top_height <= height - opening_height - bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_height: Length,
    /// Height of the slot opening. Must not be negative and not larger
    /// than [`SemiTrapezoidFromToothWidthRotBuilder::height`] minus
    /// [`SemiTrapezoidFromToothWidthRotBuilder::bottom_height`] and
    /// [`SemiTrapezoidFromToothWidthRotBuilder::top_height`]
    /// (`0 m <= opening_height <= height - top_height - bottom_height`).
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// not be negative (`bottom_side_radius >= 0 m`). Is shrunk to the maximum
    /// possible value if required by the slot geometry, see
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
    /// [`SemiTrapezoidSlot::top_radius`].
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub top_radius: Length,
    /// Radius of the fillet between the top slope and the slot sides. Must not
    /// be negative (`top_side_radius >= 0 m`). Is shrunk to the maximum
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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
        let side_height = builder.height - builder.bottom_height - builder.opening_height;
        let [bottom_side_width, top_side_width] = slot_side_bottom_and_top_width_from_rot_core(
            builder.tooth_width,
            builder.air_gap_radius,
            builder.yoke_radius,
            builder.slots,
            side_height,
            builder.opening_width,
            builder.opening_height,
        );

        let slot_angle = TAU / builder.slots as f64;

        let bottom_angle = BottomAngle::FromWidthAndHeight {
            bottom_width: builder.bottom_width,
            bottom_side_width,
            bottom_height: builder.bottom_height,
            slot_angle,
        };

        let top_angle = TopAngle::FromWidthAndHeight {
            top_width: builder.top_width,
            top_side_width,
            top_height: builder.top_height,
            slot_angle,
        };

        return SemiTrapezoidBuilder {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            slot_angle,
            bottom_angle,
            top_angle,
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

This struct can be (fallibly) converted into an [`SemiTrapezoidSlot`] via its
[`TryFrom`] / [`TryInto`] implementation. It is composed from some of the
parameters shown in the drawing below. See the field docstrings for the valid
value ranges. Even with all parameters being inside the value ranges, some
parameter combinations might still result in invalid slot outlines, in which
case this function will return an
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
use stem_slot::semi_trapezoid::SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder;

let builder = SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder {
    tooth_width: Length::new::<millimeter>(6.0),
    air_gap_radius: Length::new::<millimeter>(50.0),
    yoke_radius: Length::new::<millimeter>(80.0),
    slots: 36,
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    bottom_radius: Length::new::<millimeter>(2.0),
    top_radius: Length::new::<millimeter>(2.0),
    opening_radius: Length::new::<millimeter>(0.25),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 137.934, epsilon=1e-3);
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
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Total height of the slot. Must be larger than
    /// [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder::opening_height`]
    /// (`height > opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot opening. Must not be negative and smaller
    /// than [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder::height`]
    /// (`0 m <= opening_height < height`).
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
    /// m`). Is shrunk to the maximum possible value if required by the slot
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
    /// or the slot sides. Must not be negative (`top_radius >= 0 m`). Is shrunk
    /// to the maximum possible value if required by the slot geometry, see
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
    /// be negative (`opening_radius >= 0 m`). Is shrunk to the maximum possible
    /// value if required by the slot geometry, see
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
        let side_height = builder.height - builder.opening_height;
        let [bottom_width, top_width] = slot_side_bottom_and_top_width_from_rot_core(
            builder.tooth_width,
            builder.air_gap_radius,
            builder.yoke_radius,
            builder.slots,
            side_height,
            builder.opening_width,
            builder.opening_height,
        );
        let bottom_height = Length::new::<meter>(0.0);
        let top_height = Length::new::<meter>(0.0);
        let bottom_side_radius = Length::new::<meter>(0.0);
        let top_side_radius = Length::new::<meter>(0.0);

        return SemiTrapezoidFromToothWidthRotBuilder {
            tooth_width: builder.tooth_width,
            air_gap_radius: builder.air_gap_radius,
            yoke_radius: builder.yoke_radius,
            slots: builder.slots,
            bottom_width,
            top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            bottom_height,
            top_height,
            opening_height: builder.opening_height,
            bottom_radius: builder.bottom_radius,
            bottom_side_radius,
            top_radius: builder.top_radius,
            top_side_radius,
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
            SemiTrapezoidBuilder(SemiTrapezoidBuilder),
            SemiTrapezoidWithoutSlopesBuilder(SemiTrapezoidWithoutSlopesBuilder),
            SemiTrapezoidWithTopHeightBuilder(SemiTrapezoidWithTopHeightBuilder),
            SemiTrapezoidWithBottomHeightBuilder(SemiTrapezoidWithBottomHeightBuilder),
            SemiTrapezoidWithTopSideWidthBuilder(SemiTrapezoidWithTopSideWidthBuilder),
            SemiTrapezoidWithBottomSideWidthBuilder(SemiTrapezoidWithBottomSideWidthBuilder),
            SemiTrapezoidFromToothWidthRotBuilder(SemiTrapezoidFromToothWidthRotBuilder),
            SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder(
                SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder,
            ),
        }
        let se = SlotEnum::deserialize(deserializer)?;
        match se {
            SlotEnum::SemiTrapezoidBuilder(s) => s.try_into().map_err(serde::de::Error::custom),
            SlotEnum::SemiTrapezoidWithoutSlopesBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithTopHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithBottomHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithTopSideWidthBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithBottomSideWidthBuilder(s) => {
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
