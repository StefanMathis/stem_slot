/*!
This module defines an [`OpenTrapezoidSlot`] - a trapezoid slot which is open
towards the air gap - as well as a couple of "builder" structs which can be used
to create an [`OpenTrapezoidSlot`]. See the struct documentation for more.
 */

use compare_variables::{Comparison, ComparisonOperator, ComparisonValue, compare_variables};
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::{
    borrow::Cow,
    f64::consts::{FRAC_PI_2, PI, TAU},
};
use stem_material::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{semi_trapezoid::BottomAngleFromWidthHeight, slot::Slot};

/**
A trapezoid slot which is "open" (i.e. not closed or semi-closed towards the air
gap).

This slot type is typically found on rotary motors, using the trapezoid shape to
create teeth of constant thickness. Since the slot is open, it is especially
interesting for tooth coil windings, because the coils can be prewound and then
pushed onto the teeth (albeit at the cost of a low slot filling factor).

# Geometry and constructors

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

Not all the parameters shown in the image are needed to unequivocally describe
the slot geometry. For example, defining three of the four height parameters
directly sets the value of the fourth. Therefore, this module defines a couple
of "builder" structs which represent different possible parameter sets. These
can be fallibly converted to an [`OpenTrapezoidSlot`] via their [`TryFrom`]
implementations:
- [`OpenTrapezoidBuilder`] (builder version of [`new`](OpenTrapezoidSlot::new))
- [`OpenTrapezoidWithoutSlopesBuilder`]
- [`OpenTrapezoidWithBottomHeightBuilder`]
- [`OpenTrapezoidWithBottomSideWidthBuilder`]
- [`OpenTrapezoidWithAngleBottomBuilder`]
- [`OpenTrapezoidFromToothWidthRotBuilder`]

```
use approx;
use std::f64::consts::PI;
use stem_slot::prelude::*;
use stem_slot::open_trapezoid::OpenTrapezoidWithoutSlopesBuilder;

let builder = OpenTrapezoidWithoutSlopesBuilder {
    opening_width: Length::new::<millimeter>(5.0),
    opening_height: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(20.0),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(1.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid inputs");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 5.0, epsilon=1e-3);
```

The conversion fails if a parameter is out of bounds or if the resulting slot
outline intersects itself. The bounds of a parameter is specified in the field
docstring of the respective builder struct.

Using structs instead of constructor functions makes it less likely to confuse
arguments, since the parameter name needs to be specified explicitly. For
convenience, there exists a constructor function [`OpenTrapezoidSlot::new`]
which internally creates an [`OpenTrapezoidBuilder`] and then converts it.

# Serialization and deserialization

This struct can be directly deserialized from any of its "builder" structs (no
need for a tag). Its serialized form is that of the [`OpenTrapezoidBuilder`]
struct.

```
use approx;
use stem_slot::prelude::*;
use serde_yaml;

// Parameters of an OpenTrapezoidBuilder
let str = indoc::indoc! {"
bottom_width: 10 mm
opening_width: 5 mm
opening_height: 2 mm
bottom_width: 5 mm
height: 20 mm
side_height: 16 mm
slot_angle: PI / 18
bottom_radius: 2 mm 
slope_bottom_radius: 1 mm
consider_tooth_tip_leakage: true
"};

let slot: OpenTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 5.0, epsilon=1e-3);

// Parameters of an OpenTrapezoidWithoutSlopesBuilder
let str = indoc::indoc! {"
opening_width: 5 mm
opening_height: 2 mm
height: 20 mm
slot_angle: PI / 18
bottom_radius: 2 mm 
consider_tooth_tip_leakage: true
"};

let slot: OpenTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 5.0, epsilon=1e-3);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct OpenTrapezoidSlot {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    side_height: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    opening_height: Length,
    slot_angle: f64,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    slope_bottom_radius: Length,
    consider_tooth_tip_leakage: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    outline: Polysegment,
}

/// A helper struct for calculating some parameters of the slot.
struct CalculatedParams {
    bottom_side_width: Length,
    bottom_angle: f64,
    bottom_side_angle: f64,
}

impl OpenTrapezoidSlot {
    /**
    Creates a new [`OpenTrapezoidSlot`].

    This is the function equivalent for the [`OpenTrapezoidBuilder`] (and in
    fact creates a builder under the hood which is then converted to the final
    slot type). See the docstring of the builder struct for parameter
    descriptions.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;
    use stem_slot::prelude::*;

    let slot = OpenTrapezoidSlot::new(
        Length::new::<millimeter>(9.0),
        Length::new::<millimeter>(7.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(0.75),
        Length::new::<millimeter>(17.0),
        PI / 18.0,
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(0.0),
        true,
    ).expect("valid parameters");
    assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 140.045, epsilon=1e-3);
    ```
     */
    pub fn new(
        bottom_width: Length,
        opening_width: Length,
        height: Length,
        opening_height: Length,
        side_height: Length,
        slot_angle: f64,
        bottom_radius: Length,
        slope_bottom_radius: Length,
        consider_tooth_tip_leakage: bool,
    ) -> Result<Self, crate::error::Error> {
        OpenTrapezoidBuilder {
            bottom_width,
            opening_width,
            height,
            side_height,
            opening_height,
            slot_angle,
            bottom_radius,
            slope_bottom_radius,
            consider_tooth_tip_leakage,
        }
        .try_into()
    }

    /// Returns the width of the winding area at the intersection of the bottom
    /// slope and the slot side.
    pub fn bottom_side_width(&self) -> Length {
        return self.calculate_params().bottom_side_width;
    }

    /// Returns the slot bottom width.
    pub fn bottom_width(&self) -> Length {
        return self.bottom_width;
    }

    /// Returns the slot top width.
    pub fn top_width(&self) -> Length {
        return self.opening_width;
    }

    /// Returns the vertical height of the slot side.
    pub fn side_height(&self) -> Length {
        return self.side_height;
    }

    /// Returns the vertical of the slope at the slot bottom.
    pub fn bottom_height(&self) -> Length {
        return self.height - self.side_height - self.opening_height;
    }

    /// Returns the angle between the slot sides.
    pub fn slot_angle(&self) -> f64 {
        return self.slot_angle;
    }

    /// Returns the angle between the bottom slope and the slot bottom.
    pub fn bottom_angle(&self) -> f64 {
        return self.calculate_params().bottom_angle;
    }

    /// Returns the angle between the slot side and the bottom slope.
    pub fn bottom_side_angle(&self) -> f64 {
        return self.calculate_params().bottom_side_angle;
    }

    /// Calculates some parameters of `self`. This method is used when
    /// calculating some of the slot parameters.
    fn calculate_params(&self) -> CalculatedParams {
        let bottom_height = self.bottom_height();
        let bottom_side_width = self.opening_width
            + 2.0 * (self.height - bottom_height) / (FRAC_PI_2 - self.slot_angle / 2.0).tan();
        let delta_bottom = (bottom_side_width - self.bottom_width) / 2.0;
        let alpha = f64::from(bottom_height / delta_bottom).tan();
        let bottom_angle = alpha + FRAC_PI_2 - self.slot_angle / 2.0;
        let bottom_side_angle = 0.5 * (4.0 * PI - 2.0 * bottom_angle + self.slot_angle);

        return CalculatedParams {
            bottom_side_width,
            bottom_angle,
            bottom_side_angle,
        };
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for OpenTrapezoidSlot {
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

/**
A builder struct for an [`OpenTrapezoidSlot`] which is functionally equivalent
to [`OpenTrapezoidSlot::new`].

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::open_trapezoid::OpenTrapezoidBuilder;

let builder = OpenTrapezoidBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(7.0),
    height: Length::new::<millimeter>(17.75),
    side_height: Length::new::<millimeter>(17.0),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    slope_bottom_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 140.045, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidBuilder {
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
    /// Height of the slot. Must not be smaller than the sum of
    /// [`OpenTrapezoidBuilder::opening_height`] and
    /// [`OpenTrapezoidBuilder::side_height`] (`height >= opening_height +
    /// side_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Side height of the slot opening. Must be positive and not larger than
    /// [`OpenTrapezoidBuilder::height`] minus
    /// [`OpenTrapezoidBuilder::opening_height`] (`0 m < side_height <=
    /// height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub side_height: Length,
    /// Opening height of the slot opening. Must not be negative and not larger
    /// than [`OpenTrapezoidBuilder::height`] minus
    /// [`OpenTrapezoidBuilder::side_height`] (`0 m <= opening_height <=
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
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`slope_bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub slope_bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: OpenTrapezoidBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let opening_width = builder.opening_width;
        let height = builder.height;
        let bottom_radius = builder.bottom_radius;
        let slope_bottom_radius = builder.slope_bottom_radius;
        let opening_height = builder.opening_height;
        let side_height = builder.side_height;
        let slot_angle = builder.slot_angle;

        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < bottom_width)?;
        compare_variables!(val zero < opening_width)?;
        compare_variables!(val zero < side_height)?;
        compare_variables!(val zero <= opening_height)?;
        compare_variables!(val zero <= bottom_radius)?;
        compare_variables!(val zero <= slope_bottom_radius)?;

        // A bit of tolerance is necessary to account for floating point rounding
        // errors.
        let sum_side_opening_height = opening_height + side_height;
        if approx::ulps_ne!(
            sum_side_opening_height.get::<meter>(),
            height.get::<meter>(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            compare_variables!(height >= sum_side_opening_height)?;
        }

        // The slot height height is the sum of opening_height, side_height and
        // bottom_height
        let bottom_height = height - side_height - opening_height;
        let bottom_side_width =
            opening_width + 2.0 * (height - bottom_height) / (FRAC_PI_2 - slot_angle / 2.0).tan();

        // Points 0 - 7 as defined in [20201109_BerechnungNut.pdf]. Only the first tree
        // vertices are needed

        let v1 = [opening_width.get::<meter>() / 2.0, 0.0];
        let v2 = [
            bottom_side_width.get::<meter>() / 2.0,
            (side_height + opening_height).get::<meter>(),
        ];
        let v3 = [bottom_width.get::<meter>() / 2.0, height.get::<meter>()];

        let outline = if (side_height + opening_height) == height {
            Polysegment::from_fillet_chain(
                &[v1, v3, [-v3[0], v3[1]], [-v1[0], v1[1]]],
                &[bottom_radius.get::<meter>(), bottom_radius.get::<meter>()],
            )
        } else {
            Polysegment::from_fillet_chain(
                &[
                    v1,
                    v2,
                    v3,
                    [-v3[0], v3[1]],
                    [-v2[0], v2[1]],
                    [-v1[0], v1[1]],
                ],
                &[
                    slope_bottom_radius.get::<meter>(),
                    bottom_radius.get::<meter>(),
                    bottom_radius.get::<meter>(),
                    slope_bottom_radius.get::<meter>(),
                ],
            )
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

        return Ok(Self {
            bottom_width,
            opening_width,
            height,
            side_height,
            opening_height,
            slot_angle,
            bottom_radius,
            slope_bottom_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
            outline,
        });
    }
}

/**
A builder struct for an [`OpenTrapezoidSlot`] without slopes at the slot bottom.

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::open_trapezoid::OpenTrapezoidWithoutSlopesBuilder;

let builder = OpenTrapezoidWithoutSlopesBuilder {
    opening_width: Length::new::<millimeter>(7.0),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 149.613, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithoutSlopesBuilder {
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Height of the slot. Must not be smaller than
    /// [`OpenTrapezoidWithoutSlopesBuilder::opening_height`] (`height >=
    /// opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot. Must not be negative and not be larger than
    /// [`OpenTrapezoidWithoutSlopesBuilder::height`] (`0 m <= opening_height <=
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
    /// Radius of the fillet between the slot bottom and the slot sides. Must
    /// not be negative (`bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithoutSlopesBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithoutSlopesBuilder) -> Result<Self, Self::Error> {
        let bottom_width =
            value.opening_width + 2.0 * value.height * (0.5 * value.slot_angle).sin();
        let side_height = value.height - value.opening_height;

        return Self::new(
            bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            value.slot_angle,
            value.bottom_radius,
            Length::new::<meter>(0.0),
            value.consider_tooth_tip_leakage,
        );
    }
}

/**
A builder struct for an [`OpenTrapezoidSlot`] using the bottom instead of the
side height.

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::open_trapezoid::OpenTrapezoidWithBottomHeightBuilder;

let builder = OpenTrapezoidWithBottomHeightBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(7.0),
    height: Length::new::<millimeter>(17.75),
    bottom_height: Length::new::<millimeter>(3.0),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    slope_bottom_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 148.79, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithBottomHeightBuilder {
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
    /// Height of the slot. Must not be smaller than the sum of
    /// [`OpenTrapezoidWithBottomHeightBuilder::opening_height`] and
    /// [`OpenTrapezoidWithBottomHeightBuilder::bottom_height`] (`height >=
    /// opening_height + bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Opening height of the slot opening. Must not be negative and not larger
    /// than [`OpenTrapezoidWithBottomHeightBuilder::height`] minus
    /// [`OpenTrapezoidWithBottomHeightBuilder::bottom_height`] (`0 m <=
    /// opening_height <= height - bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Height of the bottom slope. Must not be negative and not larger than
    /// [`OpenTrapezoidWithBottomHeightBuilder::height`] minus
    /// [`OpenTrapezoidWithBottomHeightBuilder::opening_height`] (`0 m <=
    /// bottom_height <= height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`slope_bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub slope_bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithBottomHeightBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithBottomHeightBuilder) -> Result<Self, Self::Error> {
        let side_height = value.height - value.bottom_height - value.opening_height;
        return Self::new(
            value.bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            value.slot_angle,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
        );
    }
}

/**
A builder struct for an [`OpenTrapezoidSlot`] using the bottom side width
instead of the side height.

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::open_trapezoid::OpenTrapezoidWithBottomSideWidthBuilder;

let builder = OpenTrapezoidWithBottomSideWidthBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(7.0),
    bottom_side_width: Length::new::<millimeter>(9.5),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    slope_bottom_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 148.452, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithBottomSideWidthBuilder {
    /// Width of the slot bottom. Must be positive and not be larger than
    /// [`OpenTrapezoidWithBottomSideWidthBuilder::bottom_side_width`]
    /// (`0 m < bottom_width <= bottom_side_width`).
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
    /// Height of the slot. Must not be smaller than
    /// [`OpenTrapezoidWithBottomSideWidthBuilder::opening_height`] (`height >=
    /// opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot. Must not be negative and not be larger than
    /// [`OpenTrapezoidWithBottomSideWidthBuilder::height`] (`0 m <=
    /// opening_height <= height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Width of the slot where the slot sides meet the bottom slope (widest
    /// part of the slot). Must not be smaller than
    /// [`OpenTrapezoidWithBottomSideWidthBuilder::bottom_width`]
    /// (`bottom_side_width >= bottom_width`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_side_width: Length,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Radius of the fillet between the slot bottom and bottom slope (if one
    /// exists) or the slot sides. Must not be negative (`bottom_radius >= 0
    /// m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`slope_bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub slope_bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithBottomSideWidthBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: OpenTrapezoidWithBottomSideWidthBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let bottom_side_width = builder.bottom_side_width;
        compare_variables!(bottom_width <= bottom_side_width)?;
        let delta = (bottom_side_width - builder.opening_width) / 2.0;
        let side_height =
            delta * (FRAC_PI_2 - builder.slot_angle / 2.0).tan() - builder.opening_height;

        return Self::new(
            builder.bottom_width,
            builder.opening_width,
            builder.height,
            builder.opening_height,
            side_height,
            builder.slot_angle,
            builder.bottom_radius,
            builder.slope_bottom_radius,
            builder.consider_tooth_tip_leakage,
        );
    }
}

/**
A builder struct for an [`OpenTrapezoidSlot`] using the bottom side angle
instead of the side height.

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::open_trapezoid::OpenTrapezoidWithAngleBottomBuilder;

let builder = OpenTrapezoidWithAngleBottomBuilder {
    bottom_width: Length::new::<millimeter>(9.0),
    opening_width: Length::new::<millimeter>(7.0),
    bottom_angle: (0.5 * PI).into(),
    height: Length::new::<millimeter>(17.75),
    opening_height: Length::new::<millimeter>(0.75),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    slope_bottom_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 151.788, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithAngleBottomBuilder {
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
    /// Height of the slot. Must not be smaller than
    /// [`OpenTrapezoidWithAngleBottomBuilder::opening_height`] (`height >=
    /// opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the slot. Must not be negative and not be larger than
    /// [`OpenTrapezoidWithAngleBottomBuilder::height`] (`0 m <= opening_height
    /// <= height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_height: Length,
    /// Angle between the bottom slope and the slot bottom. Can be created
    /// directly from the angle value or from other geometric parameters, see
    /// the docstring of [`BottomAngleFromWidthHeight`].
    pub bottom_angle: BottomAngleFromWidthHeight,
    /// Angle between the slot sides.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64,
    /// Radius of the fillet between the slot bottom and the slot sides. Must
    /// not be negative (`bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`slope_bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub slope_bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithAngleBottomBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithAngleBottomBuilder) -> Result<Self, Self::Error> {
        let alpha = -crate::semi_trapezoid::bottom_slope_angle(
            value.bottom_angle.value(),
            value.slot_angle,
        );
        let beta = FRAC_PI_2 - 0.5 * value.slot_angle;

        let l1 = Line::from_point_angle(
            [
                0.5 * value.bottom_width.get::<meter>(),
                value.height.get::<meter>(),
            ],
            alpha,
        );
        let l2 = Line::from_point_angle([0.5 * value.opening_width.get::<meter>(), 0.0], beta);

        let intersection: [f64; 2] =
            match l1.intersections_primitive(&l2, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                PrimitiveIntersections::One(p) => p,
                _ => {
                    return Err(Comparison::new(
                        ComparisonValue::new(alpha, Some("angle of slot bottom slope")),
                        ComparisonOperator::Equal,
                        ComparisonValue::new(beta, Some("perpendicular to slot side")),
                        ComparisonOperator::Equal,
                        None,
                    )
                    .into());
                }
            };

        let side_height = Length::new::<meter>(intersection[1]) - value.opening_height;

        return Self::new(
            value.bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            value.slot_angle,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
        );
    }
}

/**
A builder struct for an [`OpenTrapezoidSlot`] in a rotary core with constant
tooth width.

This struct can be (fallibly) converted into an [`OpenTrapezoidSlot`] via its
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
    doc = "![Open trapezoid slot definitions][cad_open_trapezoid]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_open_trapezoid", "docs/img/cad_open_trapezoid.svg")
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
use stem_slot::prelude::*;
use stem_slot::open_trapezoid::OpenTrapezoidFromToothWidthRotBuilder;

let builder = OpenTrapezoidFromToothWidthRotBuilder {
    tooth_width: Length::new::<millimeter>(6.0),
    air_gap_radius: Length::new::<millimeter>(50.0),
    yoke_radius: Length::new::<millimeter>(80.0),
    slots: 36,
    opening_width: Length::new::<millimeter>(7.0),
    height: Length::new::<millimeter>(17.75),
    bottom_height: Length::new::<millimeter>(2.0),
    opening_height: Length::new::<millimeter>(2.0),
    bottom_radius: Length::new::<millimeter>(2.0),
    slope_bottom_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid parameters");
assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 147.020, epsilon=1e-3);
```
 */
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidFromToothWidthRotBuilder {
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
    /// [`OpenTrapezoidFromToothWidthRotBuilder::yoke_radius`], the slots are
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
    /// [`OpenTrapezoidFromToothWidthRotBuilder::air_gap_radius`], the slots are
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
    /// Width of the slot opening. Must be positive (`opening_width > 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub opening_width: Length,
    /// Height of the slot. Must not be smaller than the sum of
    /// [`OpenTrapezoidFromToothWidthRotBuilder::opening_height`] and
    /// [`OpenTrapezoidFromToothWidthRotBuilder::bottom_height`] (`height >=
    /// opening_height + bottom_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub height: Length,
    /// Height of the bottom slope. Must not be negative and not larger than
    /// [`OpenTrapezoidFromToothWidthRotBuilder::height`] minus
    /// [`OpenTrapezoidFromToothWidthRotBuilder::opening_height`] (`0 m <=
    /// bottom_height <= height - opening_height`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_height: Length,
    /// Opening height of the slot opening. Must not be negative and not larger
    /// than [`OpenTrapezoidFromToothWidthRotBuilder::height`] minus
    /// [`OpenTrapezoidFromToothWidthRotBuilder::bottom_height`] (`0 m <=
    /// opening_height <= height - bottom_height`).
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
    /// m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub bottom_radius: Length,
    /// Radius of the fillet between the bottom slope and the slot sides. Must
    /// not be negative (`slope_bottom_radius >= 0 m`).
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "deserialize_quantity",
            serialize_with = "serialize_quantity"
        )
    )]
    pub slope_bottom_radius: Length,
    /// If true, the tooth tip leakage is calculated using the default
    ///  implementation of [`Slot::leakage_coefficient_tooth_tip`]. Otherwise,
    /// it is set to zero.
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidFromToothWidthRotBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: OpenTrapezoidFromToothWidthRotBuilder) -> Result<Self, Self::Error> {
        let tooth_width = builder.tooth_width;
        let air_gap_radius = builder.air_gap_radius;
        let yoke_radius = builder.yoke_radius;

        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < tooth_width)?;
        compare_variables!(val zero < air_gap_radius)?;
        compare_variables!(val zero < yoke_radius)?;

        let side_height = builder.height - builder.bottom_height - builder.opening_height;
        let [bottom_width, _] = crate::slot::slot_side_bottom_and_top_width_from_rot_core(
            tooth_width,
            air_gap_radius,
            yoke_radius,
            builder.slots,
            side_height,
            builder.opening_width,
            builder.opening_height,
        );
        let slot_angle = TAU / builder.slots as f64;

        return Self::new(
            bottom_width,
            builder.opening_width,
            builder.height,
            builder.opening_height,
            side_height,
            slot_angle,
            builder.bottom_radius,
            builder.slope_bottom_radius,
            builder.consider_tooth_tip_leakage,
        );
    }
}

// =================================================================================

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for OpenTrapezoidSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum SlotEnum {
            OpenTrapezoidBuilder(OpenTrapezoidBuilder),
            OpenTrapezoidWithoutSlopesBuilder(OpenTrapezoidWithoutSlopesBuilder),
            OpenTrapezoidWithBottomHeightBuilder(OpenTrapezoidWithBottomHeightBuilder),
            OpenTrapezoidWithBottomSideWidthBuilder(OpenTrapezoidWithBottomSideWidthBuilder),
            OpenTrapezoidWithAngleBottomBuilder(OpenTrapezoidWithAngleBottomBuilder),
            OpenTrapezoidFromToothWidthRotBuilder(OpenTrapezoidFromToothWidthRotBuilder),
        }
        let s = SlotEnum::deserialize(deserializer)?;
        match s {
            SlotEnum::OpenTrapezoidBuilder(s) => s.try_into().map_err(serde::de::Error::custom),
            SlotEnum::OpenTrapezoidWithoutSlopesBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::OpenTrapezoidWithBottomHeightBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::OpenTrapezoidWithBottomSideWidthBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::OpenTrapezoidWithAngleBottomBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::OpenTrapezoidFromToothWidthRotBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}
