/*!
The `OpenTrapezoidSlot` struct represents a fully opened slot which may or may not have slopes or fillets
at the slot bottom.
*/
use compare_variables::{Comparison, ComparisonOperator, ComparisonValue, compare_variables};
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::{
    borrow::Cow,
    f64::consts::{FRAC_PI_2, TAU},
};
use stem_material::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{semi_trapezoid::AngleBottomFromWidthHeight, slot::Slot};

/**
TODO

# Constructors

The primary constructor for an [`OpenTrapezoidSlot`] is the
[`new`](OpenTrapezoidSlot::new) method, which basically takes the fields of the
struct as an arguments, sanity-checks them and then returns a struct instance.
Besides this one, the following "builder" structs are available:
- [`OpenTrapezoidBuilder`] (builder version of [`new`](OpenTrapezoidSlot::new))
- [`OpenTrapezoidWithoutSlopesBuilder`]
- [`OpenTrapezoidWithBottomHeightBuilder`]
- [`OpenTrapezoidWithBottomSideWidthBuilder`]
- [`OpenTrapezoidWithAngleBottomBuilder`]
- [`OpenTrapezoidFromToothWidthRotBuilder`]

These structs are "plain data" and all their fields are public. They are meant
to be (fallibly) converted to an [`OpenTrapezoidSlot`] via their [`TryFrom`]
implementations:

```
use approx;
use std::f64::consts::PI;
use stem_slot::prelude::*;

let builder = OpenTrapezoidWithoutSlopesBuilder {
    opening_width: Length::new::<millimeter>(5.0),
    opening_height: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(20.0),
    angle_slot: PI / 18.0,
    bottom_radius: Length::new::<millimeter>(1.0),
    consider_tooth_tip_leakage: true,
};
let slot = OpenTrapezoidSlot::try_from(builder).expect("valid inputs");
approx::assert_abs_diff_eq!(magnet.opening_width().get::<millimeter>(), 5, epsilon=1e-3);
```

# Deserialization

This struct can be deserialized from the same parameters used in
[`OpenTrapezoidSlot::new`] (see below). Besides that, all the builder structs
listed in the previous section implement [`Deserialize`], hence an
[`OpenTrapezoidSlot] can be deserialized directly from their respective
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
angle_slot: PI / 18
bottom_radius: 2 mm 
slope_bottom_radius: 1 mm
consider_tooth_tip_leakage: true
"};

let slot: OpenTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(magnet.opening_width().get::<millimeter>(), 5, epsilon=1e-3);

// Using OpenTrapezoidWithoutSlopesBuilder as an intermediate stage:
let str = indoc::indoc! {"
opening_width: 5 mm
opening_height: 2 mm
bottom_width: 5 mm
height: 20 mm
angle_slot: PI / 18
bottom_radius: 2 mm 
consider_tooth_tip_leakage: true
"};

let slot: OpenTrapezoidSlot = serde_yaml::from_str(&str).expect("valid dimensions");
approx::assert_abs_diff_eq!(magnet.opening_width().get::<millimeter>(), 5, epsilon=1e-3);
```

##

 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct OpenTrapezoidSlot {
    bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
    opening_width: Length, // Width of the slot opening
    height: Length,       // Total slot height (including slot opening)
    side_height: Length,  // Slot side height (slot height - slot opening - slopes)
    opening_height: Length, // Height of the slot opening
    angle_slot: f64,      // Angle between the slot sides and the slot bottom in degree
    bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom (opposite
                           * side of the slot opening) */
    slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
    consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                  * diagram 3.7.2 of [MVP08] or not. */
    #[cfg_attr(feature = "serde", serde(skip))]
    outline: Polysegment,
}

struct DependentParametersSlotTrapezoidOpen {
    _bottom_height: Length,
    bottom_side_width: Length,
    _angle_bottom: f64,
}

impl OpenTrapezoidSlot {
    /// This function creates a OpenTrapezoidSlot instance from various geometry
    /// parameters. For a full documentation, see
    /// [20201109_BerechnungNut.pdf]
    pub fn new(
        bottom_width: Length, // Slot width at the slot bottom (opposite side of the slot opening)
        opening_width: Length, // Width of the slot opening
        height: Length,       // Total slot height (including slot opening)
        opening_height: Length, // Height of the slot opening
        side_height: Length,  // Slot side height (slot height - slot opening - slopes)
        angle_slot: f64,      // Angle between the slot sides and the slot bottom in degree
        bottom_radius: Length, /* Edge fillet radii of the trapezoid at the slot bottom
                               * (opposite side of the slot opening) */
        slope_bottom_radius: Length, // Edge fillet radii between bottom slope and slot side
        consider_tooth_tip_leakage: bool, /* Whether to consider the tooth tip leakage according to
                                      * diagram 3.7.2 of [MVP08] or not. */
    ) -> Result<Self, crate::error::Error> {
        OpenTrapezoidBuilder {
            bottom_width,
            opening_width,
            height,
            side_height,
            opening_height,
            angle_slot,
            bottom_radius,
            slope_bottom_radius,
            consider_tooth_tip_leakage,
        }
        .try_into()
    }

    fn dependent_parameters(&self) -> DependentParametersSlotTrapezoidOpen {
        // The slot height height is the sum of opening_height, side_height and
        // bottom_height
        let bottom_height = self.height - self.side_height - self.opening_height;
        let bottom_side_width = self.opening_width
            + 2.0 * (self.height - bottom_height) / (FRAC_PI_2 - self.angle_slot / 2.0).tan();
        let delta_bottom = (bottom_side_width - self.bottom_width) / 2.0;
        let alpha = f64::from(bottom_height / delta_bottom).tan();
        let angle_bottom = alpha + FRAC_PI_2 - self.angle_slot / 2.0;

        return DependentParametersSlotTrapezoidOpen {
            _bottom_height: bottom_height,
            bottom_side_width,
            _angle_bottom: angle_bottom,
        };
    }

    pub fn bottom_side_width(&self) -> Length {
        return self.dependent_parameters().bottom_side_width;
    }

    pub fn top_side_width(&self) -> Length {
        return self.opening_width;
    }

    /// Returns slot bottom width
    pub fn bottom_width(&self) -> Length {
        let dep_params = self.dependent_parameters();
        return dep_params.bottom_side_width;
    }

    pub fn top_width(&self) -> Length {
        return self.opening_width;
    }

    pub fn side_height(&self) -> Length {
        return self.side_height;
    }

    pub fn bottom_height(&self) -> Length {
        return self.height - self.side_height - self.opening_height;
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Slot for OpenTrapezoidSlot {
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

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length, // Width of the slot opening
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length,
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
        let angle_slot = builder.angle_slot;

        let zero = Length::new::<meter>(0.0);
        compare_variables!(val zero < bottom_width)?;
        compare_variables!(val zero < opening_width)?;
        compare_variables!(val zero < height)?;
        compare_variables!(val zero < side_height)?;
        compare_variables!(val zero <= opening_height)?;
        compare_variables!(val zero <= bottom_radius)?;
        compare_variables!(val zero <= slope_bottom_radius)?;
        compare_variables!(opening_height < height)?;

        // The slot height height is the sum of opening_height, side_height and
        // bottom_height
        let bottom_height = height - side_height - opening_height;
        let bottom_side_width =
            opening_width + 2.0 * (height - bottom_height) / (FRAC_PI_2 - angle_slot / 2.0).tan();

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
            angle_slot,
            bottom_radius,
            slope_bottom_radius,
            consider_tooth_tip_leakage: builder.consider_tooth_tip_leakage,
            outline,
        });
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithoutSlopesBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithoutSlopesBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithoutSlopesBuilder) -> Result<Self, Self::Error> {
        let bottom_width =
            value.opening_width + 2.0 * value.height * (0.5 * value.angle_slot).sin();
        let side_height = value.height - value.opening_height;

        return Self::new(
            bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            value.angle_slot,
            value.bottom_radius,
            Length::new::<meter>(0.0),
            value.consider_tooth_tip_leakage,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithBottomHeightBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length,
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
            value.angle_slot,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithBottomSideWidthBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_side_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length,
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithBottomSideWidthBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithBottomSideWidthBuilder) -> Result<Self, Self::Error> {
        let delta = (value.bottom_side_width - value.opening_width) / 2.0;
        let side_height = delta * (FRAC_PI_2 - value.angle_slot / 2.0).tan() - value.opening_height;

        return Self::new(
            value.bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            value.angle_slot,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidWithAngleBottomBuilder {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    pub angle_bottom: AngleBottomFromWidthHeight,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length,
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidWithAngleBottomBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidWithAngleBottomBuilder) -> Result<Self, Self::Error> {
        let alpha = -crate::semi_trapezoid::angle_bottom_slope(
            value.angle_bottom.value(),
            value.angle_slot,
        );
        let beta = FRAC_PI_2 - 0.5 * value.angle_slot;

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
            value.angle_slot,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OpenTrapezoidFromToothWidthRotBuilder {
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
    pub bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub opening_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_radius: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub slope_bottom_radius: Length,
    pub consider_tooth_tip_leakage: bool,
}

impl TryFrom<OpenTrapezoidFromToothWidthRotBuilder> for OpenTrapezoidSlot {
    type Error = crate::error::Error;

    fn try_from(value: OpenTrapezoidFromToothWidthRotBuilder) -> Result<Self, Self::Error> {
        let side_height = value.height - value.bottom_height - value.opening_height;
        let [bottom_width, _] = crate::slot::slot_side_bottom_and_top_width_from_rot_core(
            value.tooth_width,
            value.air_gap_radius,
            value.yoke_radius,
            value.slots,
            side_height,
            value.opening_width,
            value.opening_height,
        );
        let angle_slot = TAU / value.slots as f64;

        return Self::new(
            bottom_width,
            value.opening_width,
            value.height,
            value.opening_height,
            side_height,
            angle_slot,
            value.bottom_radius,
            value.slope_bottom_radius,
            value.consider_tooth_tip_leakage,
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
