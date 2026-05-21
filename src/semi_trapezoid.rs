/*!
The `SemiTrapezoidSlot` struct represents a semi-closed slot which may or may not have slopes or radii
at the slot top or bottom. The slot top is the part where the slot opening is located.
*/
use compare_variables::{Comparison, ComparisonOperator, ComparisonValue, compare_variables};
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::{
    borrow::Cow,
    f64::consts::{FRAC_PI_2, TAU},
};
use stem_material::prelude::*;

use crate::slot::slot_side_bottom_and_top_width_from_rot_core;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::slot::Slot;

/**
TODO

# Constructors

The primary constructor for an [`SemiTrapezoidSlot`] is the
[`new`](SemiTrapezoidSlot::new) method, which basically takes the fields of the
struct as an arguments, sanity-checks them and then returns a struct instance.
Besides this one, the following "builder" structs are available:
- [`SemiTrapezoidBuilder`] (builder version of [`new`](SemiTrapezoidSlot::new))
- [`SemiTrapezoidWithoutSlopesBuilder`]
- [`SemiTrapezoidWithTopHeightBuilder`]
- [`SemiTrapezoidWithBottomHeightBuilder`]
- [`SemiTrapezoidWithSideTopWidthBuilder`]
- [`SemiTrapezoidWithSideBottomWidthBuilder`]
- [`SemiTrapezoidFromToothWidthRotBuilder`]
- [`SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder`]

These structs are "plain data" and all their fields are public. They are meant
to be (fallibly) converted to an [`SemiTrapezoidSlot`] via their [`TryFrom`]
implementations:

```
use approx;
use std::f64::consts::PI;
use stem_slot::prelude::*;

let builder = SemiTrapezoidWithoutSlopesBuilder {
    opening_width: Length::new::<millimeter>(5.0),
    opening_height: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(20.0),
    slot_angle: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(1.0),
    effective_opening_height: Some(Length::new::<millimeter>(2.0)),
    consider_tooth_tip_leakage: true,
};
let slot = SemiTrapezoidSlot::try_from(builder).expect("valid inputs");
approx::assert_abs_diff_eq!(magnet.opening_width().get::<millimeter>(), 5, epsilon=1e-3);
```

# Deserialization

This struct can be deserialized from the same parameters used in
[`SemiTrapezoidSlot::new`] (see below). Besides that, all the builder structs
listed in the previous section implement [`Deserialize`], hence an
[`SemiTrapezoidSlot] can be deserialized directly from their respective
serialized representation (without the need for a tag).

```
use approx;
use stem_slot::prelude::*;
use serde_yaml;

// Parameters from "new" method
let str = indoc::indoc! {"
opening_width: 5 mm
opening_height: 2 mm
bottom_width: 5 mm
height: 20 mm
side_height: 16 mm
effective_opening_height: 2 mm
slot_angle: PI / 18
bottom_radius: 2 mm 
slope_bottom_radius: 1 mm
consider_tooth_tip_leakage: true
"};

let slot: SemiTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(magnet.opening_width().get::<millimeter>(), 5, epsilon=1e-3);

// Using OpenTrapezoidWithoutSlopesBuilder as an intermediate stage:
let str = indoc::indoc! {"
opening_width: 5 mm
opening_height: 2 mm
bottom_width: 5 mm
height: 20 mm
effective_opening_height: 2 mm
slot_angle: PI / 18
bottom_radius: 2 mm 
consider_tooth_tip_leakage: true
"};

let slot: SemiTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(slot.opening_width().get::<millimeter>(), 5, epsilon=1e-3);
```
##

 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SemiTrapezoidSlot {
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    top_width: Length,    // Slot width at the slot top (side of the slot opening)
    opening_width: Length, // Width of the slot opening
    height: Length,       // Total slot height (including slot opening)
    side_height: Length,  // Slot side height (slot height - slot opening - slopes)
    opening_height: Length, // Height of the slot opening
    slot_angle: f64,      // Angle between the slot sides
    bottom_angle: f64,    // Angle between the slot sides and the slot bottom in degree
    angle_top: f64,       // Angle between the slot sides and the slot top in degree
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                           * side of the slot opening) */
    slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    top_radius: Length,          /* Edge fillet radii of the trapezoid at the slot top (side of
                                  * the slot opening) */
    slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    opening_radius: Length,   // Edge fillet radii of the slot opening at the slot inside
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                               * diagram 3.7.2 of [MVP08] or not. */
    #[cfg_attr(feature = "serde", serde(skip))]
    outline: Polysegment,
}

impl SemiTrapezoidSlot {
    /// This function creates a SemiTrapezoidSlot instance from various geometry
    /// parameters. For a full documentation, see
    /// [20201109_BerechnungNut.pdf]
    ///
    /// At least one of the `Option` width (prefix `b_`) or height (prefix `h_`)
    /// arguments must be given to `Some(builder)`. All other arguments can then
    /// be set to None. If both bottom_width and side_bottom_width are given
    /// AND if they are identical, bottom_angle can be omitted as well. The
    /// same goes for angle_top.
    pub fn new(
        bottom_width: Length,
        top_width: Length,
        opening_width: Length,
        height: Length,
        side_height: Length,
        opening_height: Length,
        slot_angle: f64,
        bottom_angle: f64,
        angle_top: f64,
        bottom_radius: Length,
        slope_bottom_radius: Length,
        top_radius: Length,
        slope_top_radius: Length,
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
            slope_bottom_radius,
            consider_tooth_tip_leakage,
            top_width,
            bottom_angle: bottom_angle.into(),
            angle_top: angle_top.into(),
            top_radius,
            slope_top_radius,
            opening_radius,
        }
        .try_into()
    }

    /// Returns slot bottom width
    pub fn bottom_width(&self) -> Length {
        let dep_params = self.dependent_parameters();
        return dep_params.side_bottom_width;
    }

    pub fn bottom_side_width(&self) -> Length {
        return self.dependent_parameters().side_bottom_width;
    }

    pub fn top_width(&self) -> Length {
        return self.top_width;
    }

    pub fn top_side_width(&self) -> Length {
        return self.dependent_parameters().side_top_width;
    }

    pub fn top_height(&self) -> Length {
        return self.dependent_parameters().top_height;
    }

    pub fn bottom_angle(&self) -> f64 {
        return self.bottom_angle;
    }

    pub fn angle_top(&self) -> f64 {
        return self.angle_top;
    }

    /// Helper function, not to be called directly
    fn dependent_parameters(&self) -> DependentParametersSemiTrapezoidSlot {
        let alpha = bottom_slope_angle(self.bottom_angle, self.slot_angle);
        let beta = angle_top_slope(self.angle_top, self.slot_angle);
        let delta_b_side = self.side_height * (self.slot_angle / 2.0).tan();

        /*
        Now the slope points 4 and 5 must fulfill two conditions:
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
        let side_bottom_width: Length;
        if approx::abs_diff_eq!(alpha, 0.0, epsilon = 1e-15) {
            bottom_height = Length::new::<meter>(0.0);
            side_bottom_width = self.bottom_width;
        } else {
            bottom_height = (2.0 * delta_b_side - self.bottom_width
                + self.top_width
                + 2.0 * (self.height - self.side_height - self.opening_height) / beta.tan())
                / (2.0 / alpha.tan() + 2.0 / beta.tan());
            side_bottom_width = self.bottom_width + 2.0 * bottom_height / alpha.tan();
        }

        let top_height = self.height - self.side_height - bottom_height - self.opening_height;
        let side_top_width: Length;
        if approx::abs_diff_eq!(beta, 0.0, epsilon = 1e-15) {
            side_top_width = self.top_width;
        } else {
            side_top_width = self.top_width + 2.0 * top_height / beta.tan()
        }

        return DependentParametersSemiTrapezoidSlot {
            top_height,
            side_bottom_width,
            side_top_width,
        };
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for SemiTrapezoidSlot {
    fn outline(&self) -> Cow<'_, Polysegment> {
        return Cow::Borrowed(&self.outline);
    }

    /// Returns the total slot height
    fn height(&self) -> Length {
        return self.height;
    }

    /// Returns the slot opening width
    fn opening_width(&self) -> Length {
        return self.opening_width;
    }

    /// Returns the slot opening width
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

/// Helper struct for the calculation of dependent properties, not meant for
/// external use
struct DependentParametersSemiTrapezoidSlot {
    top_height: Length,
    side_bottom_width: Length,
    side_top_width: Length,
}
pub fn angle_top_slope(angle_top: f64, slot_angle: f64) -> f64 {
    return angle_top - slot_angle / 2.0 - FRAC_PI_2;
}
pub fn bottom_slope_angle(bottom_angle: f64, slot_angle: f64) -> f64 {
    return bottom_angle + slot_angle / 2.0 - FRAC_PI_2;
}

/**
A helper struct for calculating the bottom angle of a [``]
 */
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum BottomAngleFromWidthHeight {
    Value(#[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))] f64),
    Calculate {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        bottom_width: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        side_bottom_width: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        bottom_height: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
        slot_angle: f64,
    },
}

impl BottomAngleFromWidthHeight {
    pub fn new_no_slope(slot_angle: f64) -> Self {
        return Self::Value(FRAC_PI_2 - slot_angle / 2.0);
    }

    pub fn value(&self) -> f64 {
        match self {
            BottomAngleFromWidthHeight::Value(v) => v.clone(),
            BottomAngleFromWidthHeight::Calculate {
                bottom_width,
                side_bottom_width,
                bottom_height,
                slot_angle,
            } => {
                let delta = 0.5 * (*side_bottom_width - *bottom_width);
                return bottom_height.get::<meter>().atan2(delta.get::<meter>()) + FRAC_PI_2
                    - 0.5 * slot_angle;
            }
        }
    }
}

impl From<f64> for BottomAngleFromWidthHeight {
    fn from(value: f64) -> Self {
        Self::Value(value)
    }
}

/**
Helper struct to derive the top angle from the top width, side top width, top height and the slot angle
 */
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TopAngleFromWidthHeight {
    Value(#[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))] f64),
    Calculate {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        top_width: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        side_top_width: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
        top_height: Length,
        #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
        slot_angle: f64,
    },
}

impl TopAngleFromWidthHeight {
    pub fn new_no_slope(slot_angle: f64) -> Self {
        return Self::Value(FRAC_PI_2 + slot_angle / 2.0);
    }

    pub fn value(&self) -> f64 {
        match self {
            TopAngleFromWidthHeight::Value(v) => v.clone(),
            TopAngleFromWidthHeight::Calculate {
                top_width,
                side_top_width,
                top_height,
                slot_angle,
            } => {
                let delta = 0.5 * (*side_top_width - *top_width);
                return top_height.get::<meter>().atan2(delta.get::<meter>())
                    + FRAC_PI_2
                    + 0.5 * *slot_angle;
            }
        }
    }
}

impl From<f64> for TopAngleFromWidthHeight {
    fn from(value: f64) -> Self {
        Self::Value(value)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_height: Length, // Slot side height (slot height - slot opening - slopes)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides
    pub bottom_angle: BottomAngleFromWidthHeight, /* Angle between the slot sides and the slot
                                                   * bottom in degree */
    pub angle_top: TopAngleFromWidthHeight, /* Angle between the slot sides and the slot top in
                                             * degree */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidBuilder) -> Result<Self, Self::Error> {
        let bottom_width = builder.bottom_width;
        let opening_width = builder.opening_width;
        let top_width = builder.top_width;
        let height = builder.height;
        let bottom_radius = builder.bottom_radius;
        let top_radius = builder.top_radius;
        let slope_bottom_radius = builder.slope_bottom_radius;
        let slope_top_radius = builder.slope_top_radius;
        let opening_radius = builder.opening_radius;
        let opening_height = builder.opening_height;
        let side_height = builder.side_height;
        let bottom_angle = builder.bottom_angle.value();
        let slot_angle = builder.slot_angle;
        let angle_top = builder.angle_top.value();

        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < bottom_width)?;
        compare_variables!(val zero <= opening_width)?;
        compare_variables!(val zero <= top_width)?;
        compare_variables!(val zero < height)?;
        compare_variables!(val zero <= opening_height)?;
        compare_variables!(val zero <= bottom_radius)?;
        compare_variables!(val zero <= slope_bottom_radius)?;
        compare_variables!(val zero <= top_radius)?;
        compare_variables!(val zero <= slope_top_radius)?;
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
            angle_top,
            bottom_radius,
            slope_bottom_radius,
            top_radius,
            slope_top_radius,
            opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
            outline: Polysegment::new(),
        };

        let dep_params = this.dependent_parameters();
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
        if !approx::abs_diff_eq!(
            dep_params.side_top_width.get::<meter>(),
            builder.top_width.get::<meter>(),
            epsilon = 1e-15
        ) {
            points.push([
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ]);
            radii.push(builder.slope_top_radius.get::<meter>());
        }

        // Vertex 4
        points.push([
            dep_params.side_top_width.get::<meter>() / 2.0,
            (builder.opening_height + dep_params.top_height).get::<meter>(),
        ]);
        radii.push(builder.top_radius.get::<meter>());

        // Vertex 5
        if dep_params.side_bottom_width > builder.bottom_width {
            points.push([
                dep_params.side_bottom_width.get::<meter>() / 2.0,
                (builder.opening_height + dep_params.top_height + builder.side_height)
                    .get::<meter>(),
            ]);
            radii.push(builder.slope_bottom_radius.get::<meter>());
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

        this.outline = outline;
        return Ok(this);
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithoutSlopesBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Slot opening height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithoutSlopesBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithoutSlopesBuilder) -> Result<Self, Self::Error> {
        // Calculate the top width from the bottom width and the slot side height
        let top_width = builder.bottom_width
            - 2.0 * (builder.height - builder.opening_height) * (builder.slot_angle / 2.0).tan();
        let side_height = builder.height - builder.opening_height;

        let angle_top = TopAngleFromWidthHeight::new_no_slope(builder.slot_angle);
        let bottom_angle = BottomAngleFromWidthHeight::new_no_slope(builder.slot_angle);

        return SemiTrapezoidBuilder {
            bottom_width: builder.bottom_width,
            top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            slot_angle: builder.slot_angle,
            bottom_angle,
            angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: Length::new::<meter>(0.0),
            top_radius: builder.top_radius,
            slope_top_radius: Length::new::<meter>(0.0),
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithTopHeightBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_height: Length, // Top slope height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides
    pub bottom_angle: BottomAngleFromWidthHeight, /* Angle between the slot sides and the slot
                                                   * bottom in degree */
    pub angle_top: TopAngleFromWidthHeight, /* Angle between the slot sides and the slot top in
                                             * degree */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithTopHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithTopHeightBuilder) -> Result<Self, Self::Error> {
        let gamma = angle_top_slope(builder.angle_top.value(), builder.slot_angle);
        let side_top_width = builder.top_width + 2.0 * builder.top_height / gamma.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let alpha = bottom_slope_angle(builder.bottom_angle.value(), builder.slot_angle);
        let beta = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.bottom_width.get::<meter>(),
                builder.height.get::<meter>(),
            ],
            alpha,
        );
        let l2 = Line::from_point_angle(
            [
                side_top_width.get::<meter>() / 2.0,
                (builder.opening_height + builder.top_height).get::<meter>(),
            ],
            beta,
        );

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
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithBottomHeightBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_height: Length, // Bottom slope height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides
    pub bottom_angle: BottomAngleFromWidthHeight, /* Angle between the slot sides and the slot
                                                   * bottom in degree */
    pub angle_top: TopAngleFromWidthHeight, /* Angle between the slot sides and the slot top in
                                             * degree */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithBottomHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithBottomHeightBuilder) -> Result<Self, Self::Error> {
        let alpha = bottom_slope_angle(builder.bottom_angle.value(), builder.slot_angle);
        let side_bottom_width = builder.bottom_width + 2.0 * builder.bottom_height / alpha.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 4 (side to slope_top)
        let beta = angle_top_slope(builder.angle_top.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ],
            beta,
        );
        let l2 = Line::from_point_angle(
            [
                side_bottom_width.get::<meter>() / 2.0,
                (builder.height - builder.bottom_height).get::<meter>(),
            ],
            gamma,
        );

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
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithSideTopWidthBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides
    pub bottom_angle: BottomAngleFromWidthHeight, /* Angle between the slot sides and the slot
                                                   * bottom in degree */
    pub angle_top: TopAngleFromWidthHeight, /* Angle between the slot sides and the slot top in
                                             * degree */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithSideTopWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithSideTopWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.side_top_width - builder.top_width);
        let beta = angle_top_slope(builder.angle_top.value(), builder.slot_angle);
        let top_height = delta * beta.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let alpha = -bottom_slope_angle(builder.bottom_angle.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.bottom_width.get::<meter>(),
                builder.height.get::<meter>(),
            ],
            alpha,
        );
        let l2 = Line::from_point_angle(
            [
                0.5 * builder.side_top_width.get::<meter>(),
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
                        alpha,
                    );
                    let l2 = Line::from_point_angle(
                        [
                            builder.side_top_width.get::<meter>() / 2.0,
                            (builder.opening_height + top_height).get::<meter>(),
                        ],
                        gamma,
                    );

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
                    }
                }
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
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidWithSideBottomWidthBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_bottom_width: Length, // Bottom slope height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub slot_angle: f64, // Angle between the slot sides
    pub bottom_angle: BottomAngleFromWidthHeight, /* Angle between the slot sides and the slot
                                                   * bottom in degree */
    pub angle_top: TopAngleFromWidthHeight, /* Angle between the slot sides and the slot top in
                                             * degree */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                                * (opposite side of the slot
                                * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the
                             * slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    pub consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according
                                           * to
                                           * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithSideBottomWidthBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithSideBottomWidthBuilder) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.side_bottom_width - builder.bottom_width);
        let alpha = bottom_slope_angle(builder.bottom_angle.value(), builder.slot_angle);
        let bottom_height = delta * alpha.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let beta = angle_top_slope(builder.angle_top.value(), builder.slot_angle);
        let gamma = FRAC_PI_2 - builder.slot_angle / 2.0;

        let l1 = Line::from_point_angle(
            [
                0.5 * builder.side_bottom_width.get::<meter>(),
                (builder.height - bottom_height).get::<meter>(),
            ],
            gamma,
        );
        let l2 = Line::from_point_angle(
            [
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ],
            beta,
        );

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
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidFromToothWidthRotBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub tooth_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub air_gap_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub yoke_radius: Length,
    pub slots: u16,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length,
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<SemiTrapezoidFromToothWidthRotBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidFromToothWidthRotBuilder) -> Result<Self, Self::Error> {
        let side_height = builder.height - builder.bottom_height - builder.opening_height;
        let [side_bottom_width, side_top_width] = slot_side_bottom_and_top_width_from_rot_core(
            builder.tooth_width,
            builder.air_gap_radius,
            builder.yoke_radius,
            builder.slots,
            side_height,
            builder.opening_width,
            builder.opening_height,
        );

        let slot_angle = TAU / builder.slots as f64;

        let bottom_angle = BottomAngleFromWidthHeight::Calculate {
            bottom_width: builder.bottom_width,
            side_bottom_width,
            bottom_height: builder.bottom_height,
            slot_angle,
        };

        let angle_top = TopAngleFromWidthHeight::Calculate {
            top_width: builder.top_width,
            side_top_width,
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
            angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .try_into();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SemiTrapezoidFromToothWidthRotWithoutSlopesBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub tooth_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub air_gap_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub yoke_radius: Length,
    pub slots: u16,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_radius: Length,
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
        let slope_bottom_radius = Length::new::<meter>(0.0);
        let slope_top_radius = Length::new::<meter>(0.0);

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
            slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius,
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
            SemiTrapezoidWithSideTopWidthBuilder(SemiTrapezoidWithSideTopWidthBuilder),
            SemiTrapezoidWithSideBottomWidthBuilder(SemiTrapezoidWithSideBottomWidthBuilder),
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
            SlotEnum::SemiTrapezoidWithSideTopWidthBuilder(s) => {
                s.try_into().map_err(serde::de::Error::custom)
            }
            SlotEnum::SemiTrapezoidWithSideBottomWidthBuilder(s) => {
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
