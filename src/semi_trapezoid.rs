/*!
The `SemiTrapezoidSlot` struct represents a semi-closed slot which may or may not have slopes or fillets
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

use crate::slot::{
    angle_bottom_no_slope, angle_bottom_slope, angle_top_no_slope, angle_top_slope,
    serde_impl::{
        deserialize_angle_bottom_from_width_height, deserialize_angle_top_from_width_height,
    },
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::slot::Slot;

/// Trapezoid semi-open slot according [20201109_BerechnungNut.pdf], possibly
/// including slopes and fillets
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SemiTrapezoidSlot {
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    top_width: Length,    // Slot width at the slot top (side of the slot opening)
    opening_width: Length, // Width of the slot opening
    height: Length,       // Total slot height (including slot opening)
    side_height: Length,  // Slot side height (slot height - slot opening - slopes)
    opening_height: Length, // Height of the slot opening
    angle_slot: f64,      // Angle between the slot sides
    angle_bottom: f64,    // Angle between the slot sides and the slot bottom in degree
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
    polysegment: Polysegment,
}

impl SemiTrapezoidSlot {
    /// This function creates a SemiTrapezoidSlot instance from various geometry
    /// parameters. For a full documentation, see
    /// [20201109_BerechnungNut.pdf]
    ///
    /// At least one of the `Option` width (prefix `b_`) or height (prefix `h_`)
    /// arguments must be given to `Some(builder)`. All other arguments can then
    /// be set to None. If both bottom_width and side_bottom_width are given
    /// AND if they are identical, angle_bottom can be omitted as well. The
    /// same goes for angle_top.
    pub fn new(
        bottom_width: Length,
        top_width: Length,
        opening_width: Length,
        height: Length,
        side_height: Length,
        opening_height: Length,
        angle_slot: f64,
        angle_bottom: f64,
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
            angle_slot,
            bottom_radius,
            slope_bottom_radius,
            consider_tooth_tip_leakage,
            top_width,
            angle_bottom,
            angle_top,
            top_radius,
            slope_top_radius,
            opening_radius,
        }
        .try_into()
    }

    /// Helper function, not to be called directly
    fn dependent_parameters(&self) -> DependentParametersSemiTrapezoidSlot {
        let alpha = angle_bottom_slope(self.angle_bottom, self.angle_slot);
        let beta = angle_top_slope(self.angle_top, self.angle_slot);
        let delta_b_side = self.side_height * (self.angle_slot / 2.0).tan();

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
            _bottom_height: bottom_height,
            top_height,
            side_bottom_width,
            side_top_width,
        };
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for SemiTrapezoidSlot {
    /// Returns the total slot height
    fn height(&self) -> Length {
        return self.height;
    }

    /// Returns slot bottom width
    fn bottom_width(&self) -> Length {
        let dep_params = self.dependent_parameters();
        return dep_params.side_bottom_width;
    }

    /// Returns the slot opening width
    fn opening_width(&self) -> Length {
        return self.opening_width;
    }

    /// Returns the slot opening width
    fn opening_height(&self) -> Length {
        return self.opening_height;
    }

    /// Returns the effective magnetic slot opening height.
    fn magnetic_opening_height(&self) -> Length {
        return self.opening_height;
    }

    /// Mean slot width
    fn mean_width(&self) -> Length {
        let deps = self.dependent_parameters();
        return (deps.side_bottom_width + deps.side_top_width) / 2.0;
    }

    fn consider_tooth_tip_leakage(&self) -> bool {
        return self.consider_tooth_tip_leakage;
    }

    fn bottom_side_width(&self) -> Length {
        return self.dependent_parameters().side_bottom_width;
    }

    fn top_width(&self) -> Length {
        return self.top_width;
    }

    fn top_side_width(&self) -> Length {
        return self.dependent_parameters().side_top_width;
    }

    fn top_height(&self) -> Length {
        return self.dependent_parameters().top_height;
    }
}

/// Helper struct for the calculation of dependent properties, not meant for
/// external use
struct DependentParametersSemiTrapezoidSlot {
    _bottom_height: Length,
    top_height: Length,
    side_bottom_width: Length,
    side_top_width: Length,
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
    pub angle_slot: f64, // Angle between the slot sides
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_bottom_from_width_height")
    )]
    pub angle_bottom: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_top_from_width_height")
    )]
    pub angle_top: f64, // Angle between the slot sides and the slot top in degree
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
        let mut pv: Vec<Point2<f64>> = Vec::with_capacity(7);
        let mut pf: Vec<f64> = Vec::with_capacity(7);

        let dep_params = self.dependent_parameters();

        // Vertex 1
        if self.opening_width.get::<meter>() > 0.0 {
            pv.push(Point2::new(self.opening_width.get::<meter>() / 2.0, 0.0));
            pf.push(0.0);
        }

        // Vertex 2
        pv.push(Point2::new(
            self.opening_width.get::<meter>() / 2.0,
            self.opening_height.get::<meter>(),
        ));
        pf.push(self.opening_radius.get::<meter>());

        // Vertex 3
        if !approx::abs_diff_eq!(
            dep_params.side_top_width.get::<meter>(),
            self.top_width.get::<meter>(),
            epsilon = 1e-15
        ) {
            pv.push(Point2::new(
                self.top_width.get::<meter>() / 2.0,
                self.opening_height.get::<meter>(),
            ));
            pf.push(self.slope_top_radius.get::<meter>());
        }

        // Vertex 4
        pv.push(Point2::new(
            dep_params.side_top_width.get::<meter>() / 2.0,
            (self.opening_height + dep_params.top_height).get::<meter>(),
        ));
        pf.push(self.top_radius.get::<meter>());

        // Vertex 5
        if dep_params.side_bottom_width > self.bottom_width {
            pv.push(Point2::new(
                dep_params.side_bottom_width.get::<meter>() / 2.0,
                (self.opening_height + dep_params.top_height + self.side_height).get::<meter>(),
            ));
            pf.push(self.slope_bottom_radius.get::<meter>());
        }

        // Vertex 6
        pv.push(Point2::new(
            self.bottom_width.get::<meter>() / 2.0,
            self.height.get::<meter>(),
        ));
        pf.push(self.bottom_radius.get::<meter>());

        // Mirror the vertices and codes along the y-axis
        let mut pv_mirror = pv.clone();
        pv_mirror.reverse();
        free_functions::line_reflection(
            &mut pv_mirror,
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 1.0),
        );
        pv.append(&mut pv_mirror);

        let mut pf_mirror = pf.clone();
        pf_mirror.reverse();
        pf.append(&mut pf_mirror);

        return SegmentChain::from_fillets(pv, pf, false, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
            .unwrap();
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewWithoutSlopes {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length, // Slot opening height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    angle_slot: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                            * side of the slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the slot
                         * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                       * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<NewWithoutSlopes> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: NewWithoutSlopes) -> Result<Self, Self::Error> {
        // Calculate the top width from the bottom width and the slot side height
        let top_width = builder.bottom_width
            - 2.0 * (builder.height - builder.opening_height) * (builder.angle_slot / 2.0).tan();
        let side_height = builder.height - builder.opening_height;

        let angle_top = angle_top_no_slope(builder.angle_slot);
        let angle_bottom = angle_bottom_no_slope(builder.angle_slot);

        return SemiTrapezoidBuilder {
            bottom_width: builder.bottom_width,
            top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            angle_slot: builder.angle_slot,
            angle_bottom,
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
    pub angle_slot: f64, // Angle between the slot sides
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_bottom_from_width_height")
    )]
    pub angle_bottom: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_top_from_width_height")
    )]
    pub angle_top: f64, // Angle between the slot sides and the slot top in degree
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
        let gamma = angle_top_slope(builder.angle_top, builder.angle_slot);
        let side_top_width = builder.top_width + 2.0 * builder.top_height / gamma.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let alpha = angle_bottom_slope(builder.angle_bottom, builder.angle_slot);
        let beta = FRAC_PI_2 - builder.angle_slot / 2.0;

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
            angle_slot: builder.angle_slot,
            angle_bottom: builder.angle_bottom,
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
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_height: Length, // Bottom slope height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    angle_slot: f64, // Angle between the slot sides
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_bottom_from_width_height")
    )]
    angle_bottom: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_top_from_width_height")
    )]
    angle_top: f64, // Angle between the slot sides and the slot top in degree
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                            * side of the slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the slot
                         * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                       * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<SemiTrapezoidWithBottomHeightBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: SemiTrapezoidWithBottomHeightBuilder) -> Result<Self, Self::Error> {
        let alpha = angle_bottom_slope(builder.angle_bottom, builder.angle_slot);
        let side_bottom_width = builder.bottom_width + 2.0 * builder.bottom_height / alpha.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 4 (side to slope_top)
        let beta = angle_top_slope(builder.angle_top, builder.angle_slot);
        let gamma = FRAC_PI_2 - builder.angle_slot / 2.0;

        let first = Line::from_point_angle(
            Point2::new(
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ),
            beta,
        );
        let second = Line::from_point_angle(
            Point2::new(
                side_bottom_width.get::<meter>() / 2.0,
                (builder.height - builder.bottom_height).get::<meter>(),
            ),
            gamma,
        );

        let intersection = first
            .intersection(&second, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
            .ok_or::<crate::error::Error>(planar_geo::error::ErrorType::NotSimpleShape.into())?;

        let top_height = Length::new::<meter>(intersection.y) - builder.opening_height;
        let side_height =
            builder.height - top_height - builder.bottom_height - builder.opening_height;

        return Self {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            angle_slot: builder.angle_slot,
            angle_bottom: builder.angle_bottom,
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .check();
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "SemiTrapezoidSlot",
    fn_name = "new_with_side_top_width",
    error = "crate::error::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct NewWithSideTopWidth {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    side_top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    angle_slot: f64, // Angle between the slot sides
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_bottom_from_width_height")
    )]
    angle_bottom: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_top_from_width_height")
    )]
    angle_top: f64, // Angle between the slot sides and the slot top in degree
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                            * side of the slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the slot
                         * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                       * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<NewWithSideTopWidth> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: NewWithSideTopWidth) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.side_top_width - builder.top_width);
        let beta = angle_top_slope(builder.angle_top, builder.angle_slot);
        let top_height = delta * beta.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let alpha = -angle_bottom_slope(builder.angle_bottom, builder.angle_slot);
        let gamma = FRAC_PI_2 - builder.angle_slot / 2.0;

        let first = Line::from_point_angle(
            Point2::new(
                0.5 * builder.bottom_width.get::<meter>(),
                builder.height.get::<meter>(),
            ),
            alpha,
        );
        let second = Line::from_point_angle(
            Point2::new(
                0.5 * builder.side_top_width.get::<meter>(),
                (builder.opening_height + top_height).get::<meter>(),
            ),
            gamma,
        );

        let intersection = match first.intersection(&second, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
            Some(pt) => pt,
            None => {
                let first = Line::from_point_angle(
                    Point2::new(
                        builder.bottom_width.get::<meter>() / 2.0,
                        builder.height.get::<meter>(),
                    ),
                    alpha,
                );
                let second = Line::from_point_angle(
                    Point2::new(
                        builder.side_top_width.get::<meter>() / 2.0,
                        (builder.opening_height + top_height).get::<meter>(),
                    ),
                    gamma,
                );

                first
                    .intersection(&second, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
                    .ok_or::<crate::error::Error>(
                        planar_geo::error::ErrorType::NotSimpleShape.into(),
                    )?
                    .clone()
            }
        };

        let bottom_height = builder.height - Length::new::<meter>(intersection.y);
        let side_height = builder.height - top_height - bottom_height - builder.opening_height;

        return Self {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            angle_slot: builder.angle_slot,
            angle_bottom: builder.angle_bottom,
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .check();
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "SemiTrapezoidSlot",
    fn_name = "new_with_side_bottom_width",
    error = "crate::error::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct NewWithSideBottomWidth {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_width: Length, // Slot width at the slot top (side of the slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length, // Total slot height (including slot opening)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    side_bottom_width: Length, // Bottom slope height
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length, // Height of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    angle_slot: f64, // Angle between the slot sides
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_bottom_from_width_height")
    )]
    angle_bottom: f64, // Angle between the slot sides and the slot bottom in degree
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_angle_top_from_width_height")
    )]
    angle_top: f64, // Angle between the slot sides and the slot top in degree
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                            * side of the slot opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length, /* Edge fillet radii of the trapezoid at the slot top (side of the slot
                         * opening) */
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_top_radius: Length, // Edge fillet radii between top slope and slot side
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length, // Edge fillet radii of the slot opening at the slot inside
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                       * diagram 3.7.2 of [MVP08] or not. */
}

impl TryFrom<NewWithSideBottomWidth> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: NewWithSideBottomWidth) -> Result<Self, Self::Error> {
        let delta = 0.5 * (builder.side_bottom_width - builder.bottom_width);
        let alpha = angle_bottom_slope(builder.angle_bottom, builder.angle_slot);
        let bottom_height = delta * alpha.tan();

        // Construct two line equations with incline and one point.
        // Then find the intersection, it equals point 5 (side to slope_bottom)
        let beta = angle_top_slope(builder.angle_top, builder.angle_slot);
        let gamma = FRAC_PI_2 - builder.angle_slot / 2.0;

        let first = Line::from_point_angle(
            Point2::new(
                0.5 * builder.side_bottom_width.get::<meter>(),
                (builder.height - bottom_height).get::<meter>(),
            ),
            gamma,
        );
        let second = Line::from_point_angle(
            Point2::new(
                builder.top_width.get::<meter>() / 2.0,
                builder.opening_height.get::<meter>(),
            ),
            beta,
        );

        let intersection = first
            .intersection(&second, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
            .ok_or::<crate::error::Error>(planar_geo::error::ErrorType::NotSimpleShape.into())?;

        let top_height = Length::new::<meter>(intersection.y) - builder.opening_height;
        let side_height = builder.height - top_height - bottom_height - builder.opening_height;

        return Self {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            angle_slot: builder.angle_slot,
            angle_bottom: builder.angle_bottom,
            angle_top: builder.angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .check();
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "SemiTrapezoidSlot",
    fn_name = "new_from_tooth_width_rot",
    error = "crate::error::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct OpenTrapezoidFromToothWidthRotBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    tooth_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    air_gap_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    yoke_radius: Length,
    slots: u16,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    slope_top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length,
    consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidFromToothWidthRotBuilder> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: OpenTrapezoidFromToothWidthRotBuilder) -> Result<Self, Self::Error> {
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

        let angle_slot = TAU / builder.slots as f64;
        let angle_bottom = angle_bottom_from_width_height(
            builder.bottom_width,
            side_bottom_width,
            builder.bottom_height,
            angle_slot,
        );
        let angle_top = angle_top_from_width_height(
            builder.top_width,
            side_top_width,
            builder.top_height,
            angle_slot,
        );

        return Self {
            bottom_width: builder.bottom_width,
            top_width: builder.top_width,
            opening_width: builder.opening_width,
            height: builder.height,
            side_height,
            opening_height: builder.opening_height,
            angle_slot,
            angle_bottom,
            angle_top,
            bottom_radius: builder.bottom_radius,
            slope_bottom_radius: builder.slope_bottom_radius,
            top_radius: builder.top_radius,
            slope_top_radius: builder.slope_top_radius,
            opening_radius: builder.opening_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
        }
        .check();
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "SemiTrapezoidSlot",
    fn_name = "new_from_tooth_width_without_slopes_rot",
    error = "crate::error::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewFromToothWidthWithoutSlopesRot {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    tooth_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    air_gap_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    yoke_radius: Length,
    slots: u16,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    top_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    opening_radius: Length,
    consider_tooth_tip_leakage: bool,
}

impl TryFrom<NewFromToothWidthWithoutSlopesRot> for SemiTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(builder: NewFromToothWidthWithoutSlopesRot) -> Result<Self, Self::Error> {
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

        return OpenTrapezoidFromToothWidthRotBuilder {
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
impl<'de> Deserialize<'de> for OpenTrapezoidSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum SlotEnum {
            OpenTrapezoidBuilder(OpenTrapezoidBuilder),
            OpenTrapezoidWithoutSlopeBuilder(OpenTrapezoidWithoutSlopeBuilder),
            OpenTrapezoidWithBottomHeightBuilder(OpenTrapezoidWithBottomHeightBuilder),
            OpenTrapezoidWithBottomSideWidthBuilder(OpenTrapezoidWithBottomSideWidthBuilder),
            OpenTrapezoidWithAngleBottomBuilder(OpenTrapezoidWithAngleBottomBuilder),
            OpenTrapezoidFromToothWidthRotBuilder(OpenTrapezoidFromToothWidthRotBuilder),
        }
        let s = SlotEnum::deserialize(deserializer)?;
        match s {
            SlotEnum::OpenTrapezoidBuilder(s) => s.try_into().map_err(serde::de::Error::custom),
            SlotEnum::OpenTrapezoidWithoutSlopeBuilder(s) => {
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
