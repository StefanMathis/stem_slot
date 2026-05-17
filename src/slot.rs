/*!
This module offers the [`Slot`] trait and a couple of helper functions.

The most important item in this module is the [`Slot`] trait, around which the
entire crate is centered. The [`LayerOutlines`] struct is an iterator which is
returned by [`Slot::layer_outlines`]. The free function
[`leakage_coefficient_tooth_tip`] contains the default implementation of
[`Slot::leakage_coefficient_tooth_tip`] and is made available so it can be used
as part of custom implementations for the trait method.
[`semi_regular_polygon_side_length`] is a helper method for defining a
semi-regular polygon.
 */
#![deny(missing_docs)]

use akima_spline::AkimaSpline;
use approx::ulps_eq;
use dyn_clone::DynClone;
use gauss_quad;
use nalgebra::DMatrix;
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::any::Any;
use std::borrow::Cow;
use std::f64::consts::TAU;
use stem_material::prelude::*;

use crate::coil_layout::CoilLayout;
use crate::current_displacement::CurrentDisplacementCalculator;

/**
A trait for defining slots (grooves on the air gap side of magnetic cores).

This trait provides a simple interface for defining a slot: A groove on the air
gap side of the magnetic core which holds one or multiple coils of a winding.
The design of a slot typically strives to meet a compromise between maximizing
the available space for copper (i.e. reducing ohmic losses) and allowing for
enough space between them to not hinder the magnetic flux.

This trait offers methods for calculating the slot leakage inductance for
different [`CoilLayout`]s (see e.g.
[`Slot::mutual_inductance_leakage_coefficient`]) or the current displacement
coefficients via [`Slot::current_displacement_coefficients`].

# Implementation

Implementing the trait requires the definition of a couple simple methods
like e.g. [`Slot::outline`] describing the geometry extents of the slot.
The image below gives an overview over the definitions and conventions which
need to be followed:
*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Slot geometry definitions][cad_slot_defs]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_slot_defs", "docs/img/cad_slot_defs.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
- The air gap border of the core is on the x-axis. If the slot is "open", the
start and end points of its [`outline`](Slot::outline) must therefore be on
the x-axis as well. If the slot is closed, its start and end point must be
identical and must have a positive y-value. All segments of the outline must
have positive y-values as well.
- If the slot is open, the distance between the start and end points of the
outline is the [`Slot::opening_width`].
- Slots may have an "opening" space, where no coils are located. This space is
defined by the [`Slot::opening_height`]. All other space which is enclosed by
the slot outline (and the x-axis) is the "coil" space.
- The total y-extent of the slot is the [`Slot::height`].
- The area close to the air gap is called the "slot top", the area furthest away
from it is the "slot bottom".
- If the slot is not symmetrical about the y-axis, [`Slot::slices`] must be
overwritten. See its docstring for more.

# Example

The following code snippet shows how a simple rectangular slot like the one used
in the example image can implement [`Slot`] (this is in fact quite similar to
how [`RectangularSlot`](crate::rectangular::RectangularSlot) is implemented). In
the example, the `serde` feature is enabled, necessitating the implementation
of `Deserialize` and `Serialize`.

```
use std::borrow::Cow;

use planar_geo::prelude::*;
use serde::{Deserialize, Serialize};
use stem_slot::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MyRectangularSlot {
    width: Length,
    opening_width: Length,
    height: Length,
    opening_height: Length,
}

#[typetag::serde] // <-- Needed because of the trait definition
impl Slot for MyRectangularSlot {
    fn opening_width(&self) -> Length {
        return self.opening_width;
    }

    fn opening_height(&self) -> Length {
        return self.opening_height;
    }

    fn outline(&self) -> Cow<'_, Polysegment> {
        // Simple outline definition of a rectangular slot
        let mut pts = Vec::new();
        pts.push([
            -self.opening_width.get::<meter>() / 2.0,
            0.0,
        ]);
        pts.push([
            -self.opening_width.get::<meter>() / 2.0,
            self.opening_height.get::<meter>(),
        ]);
        pts.push([
            -self.width.get::<meter>() / 2.0,
            self.opening_height.get::<meter>(),
        ]);
        pts.push([
            -self.width.get::<meter>() / 2.0,
            self.height.get::<meter>(),
        ]);
        pts.push([
            self.width.get::<meter>() / 2.0,
            self.height.get::<meter>(),
        ]);
        pts.push([
            self.width.get::<meter>() / 2.0,
            self.opening_height.get::<meter>(),
        ]);
        pts.push([
            self.opening_width.get::<meter>() / 2.0,
            self.opening_height.get::<meter>(),
        ]);
        pts.push([
            self.opening_width.get::<meter>() / 2.0,
            0.0,
        ]);
        return Cow::Owned(Polysegment::from_points(&pts));
    }
}
```
 */
#[cfg_attr(feature = "serde", typetag::serde)]
pub trait Slot: Send + Sync + std::fmt::Debug + DynClone + Any + 'static {
    /**
    Returns the slot opening width.

    If the slot opening is parallel-sided, this value simply equals the distance
    between both sides. If it is not, it should be the mean distance between the
    sides (see also [`Slot::leakage_coefficient_opening`]). If the slot is
    closed (i.e. its outline is not connected to the air gap), this value is
    zero. See the [`Slot`] docstring for more.
     */
    fn opening_width(&self) -> Length;

    /**
    Returns the slot opening height.

    See the [`Slot`] docstring for more.
     */
    fn opening_height(&self) -> Length;

    /**
    Returns the outline of the slot.

    This is the cross-sectional outline of the slot in the x-y plane as
    defined in the [trait docstring](Slot). The returned [`Polysegment`] must
    use the coordinate system shown in the trait docstring image. It is
    assumed that this outline is constant along the entire length of the
    magnetic core.

    Some implementors of [`Slot`] may construct their outline eagerly during
    initialization, while others may construct it on demand. Returning [`Cow`]
    allows implementations to either return a borrowed precomputed outline or an
    owned value created lazily.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");
    let outline = slot.outline();
    assert_abs_diff_eq!(outline.length(), 0.054);
    ```
     */
    fn outline(&self) -> Cow<'_, Polysegment>;

    // =========================================================================

    /**
    Returns the total height of the slot.

    The default implementation returns the height of the bounding box of
    [`Slot::outline`].

    # Examples

    ```
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");
    assert_eq!(slot.height().get::<millimeter>(), 20.0);
    ```
     */
    fn height(&self) -> Length {
        let bb = self.outline().bounding_box();
        return Length::new::<meter>(bb.height());
    }

    /**
    Return if the slot is open (to the air gap).

    The slot is open if [`Slot::opening_width`] and [`Slot::opening_height`]
    are larger than zero.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use stem_slot::prelude::*;

    assert!(RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs").is_open());

    assert!(!RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(0.0),
        true,
    ).expect("valid inputs").is_open());

    assert!(!RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs").is_open());
    ```
     */
    fn is_open(&self) -> bool {
        return self.opening_width().get::<meter>() > 0.0
            && self.opening_height().get::<meter>() > 0.0;
    }

    /**
    Returns the area covered by the slot.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");

    assert_abs_diff_eq!(slot.area().get::<square_millimeter>(), 192.0);
    ```
     */
    fn area(&self) -> Area {
        let contour: Contour = self.outline().into_owned().into();
        return Area::new::<square_meter>(contour.area());
    }

    /**
    Returns [`Slot::outline`] with the slot opening being removed.

    This method "cuts off" the slot opening from [`Slot::outline`] and therefore
    returns the part of the slot outline which touches the "winding area", i.e.
    the space where the conductors / coils are located.

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use planar_geo::prelude::ToBoundingBox;
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");

    let outline = slot.outline();
    assert_abs_diff_eq!(outline.bounding_box().height(), 0.02);

    let outline_winding_area = slot.outline_winding_area();
    assert_abs_diff_eq!(outline_winding_area.bounding_box().height(), 0.018);
    ```
     */
    fn outline_winding_area(&self) -> Polysegment {
        if !self.is_open() {
            return self.outline().into_owned();
        }

        let contour: Contour = self.outline().into_owned().into();

        let bb = contour.bounding_box();

        // Identify the beginning of the slot opening
        let verts_par = [
            [2.0 * bb.xmin(), self.opening_height().get::<meter>()],
            [2.0 * bb.xmax(), self.opening_height().get::<meter>()],
        ];
        let parallel_line = Polysegment::from_points(verts_par.as_slice());

        // Cut off the slot opening. The upper part of the slot is the second item in
        // separated_lines. If the length of separated_lines is smaller than 2,
        // the contour has no slot opening and is therefore not changed.
        let separated_lines: Vec<Polysegment> =
            contour.intersection_cut(&parallel_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

        // Combine all lines which are not below self.opening_height() into a new chain.
        // The "1e-9" is necessary because of floating point rounding errors.
        // 1e-9 are 1.0 nm and therefore well out of the range of any feasible geometry
        let opening_height = self.opening_height().get::<meter>();
        return separated_lines
            .into_iter()
            .filter(|e| e.bounding_box().ymin() >= opening_height - 1e-9)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap_or(Polysegment::new());
    }

    /**
    Returns the total area available for winding layers (i.e. [`Slot::area`]
    minus the slot opening area).

    # Examples

    ```
    use approx::assert_abs_diff_eq;
    use stem_slot::prelude::*;

    let slot = RectangularSlot::new(
        Length::new::<millimeter>(10.0),
        Length::new::<millimeter>(6.0),
        Length::new::<millimeter>(20.0),
        Length::new::<millimeter>(2.0),
        true,
    ).expect("valid inputs");

    assert_abs_diff_eq!(slot.winding_area().get::<square_millimeter>(), 180.0);
    ```
     */
    fn winding_area(&self) -> Area {
        return Area::new::<square_meter>(Contour::from(self.outline_winding_area()).area());
    }

    /// Returns all parts of [`Slot::outline_winding_area`] which borders the
    /// specified `layer`.
    ///
    /// Depending on the `coil_layout` a `layer` might either border a single,
    /// continuous section of [`Slot::outline_winding_area`] or multiple parts
    /// of it (see image below). The returned [`LayerOutlines`] struct is an
    /// iterator over all parts of the outline which border `layer`.
    #[doc = ""]
    #[cfg_attr(feature = "doc-images", doc = "![Layer outlines][layer_outlines]")]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image("layer_outlines", "docs/img/layer_outlines.svg")
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// # Panics
    /// Panics if `layer` is not smaller than the [`CoilLayout::layers`] value
    /// of the given `coil_layout`.
    ///
    /// # Examples
    ///
    /// ```
    /// use approx::assert_abs_diff_eq;
    /// use stem_slot::prelude::*;
    ///
    /// // Open slot
    /// let slot = RectangularSlot::new(
    ///     Length::new::<millimeter>(10.0),
    ///     Length::new::<millimeter>(10.0),
    ///     Length::new::<millimeter>(20.0),
    ///     Length::new::<millimeter>(1.0),
    ///     true,
    /// ).expect("valid inputs");
    ///
    /// // Two outlines as shown in the drawing above
    /// assert_eq!(slot.layer_outlines(1, &CoilLayout::DoubleVertical).count(), 2);
    ///
    /// // Sum up length of all outlines
    /// assert_abs_diff_eq!(
    ///     slot.layer_outlines(1, &CoilLayout::DoubleVertical).length().get::<millimeter>(),
    ///     19.0, epsilon = 1e-6
    /// ); // Winding area height = 19 mm, divided by two because double layer, times two because two sides
    /// ```
    fn layer_outlines(&self, layer: u16, coil_layout: &CoilLayout) -> LayerOutlines {
        let polysegment = self.outline_winding_area();
        let centroid = Contour::new(polysegment.clone()).centroid();
        let layer_bounds = layer_bounds(
            self,
            layer,
            coil_layout,
            centroid,
            &polysegment.bounding_box(),
            1.0,
            1.0,
        );

        // Sum up all parts of the segment chain which are within bounds
        return LayerOutlines {
            inner: polysegment
                .intersection_cut(
                    &Polysegment::from(&layer_bounds),
                    DEFAULT_EPSILON,
                    DEFAULT_MAX_ULPS,
                )
                .into_iter(),
            layer_bounds,
        };
    }

    /// Returns the contours for all layers defined by the `coil_layout`.
    ///
    /// This method derives the contour of the entire slot from
    /// [`Slot::outline`] if `include_slot_opening` is true, otherwise from
    /// [`Slot::outline_winding_area`]. This contour is then separated into
    /// multiple sections (one per layer), which are returned in the order of
    /// the layers: `slot.layer_contours(...)[0]` corresponds to layer 0,
    /// `slot.layer_contours(...)[1]` corresponds to layer 1 and so on. The
    /// following image shows this separation using a [`CoilLayout::Quadruple`]
    /// with `include_slot_opening = true`:
    #[doc = ""]
    #[cfg_attr(feature = "doc-images", doc = "![Layer contours][layer_contours]")]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image("layer_contours", "docs/img/layer_contours.svg")
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// For convenience, [`Slot::drawables`] wraps this method and adds a
    /// [`Style`] so the contours can be drawn directly.
    ///
    /// In case of [`CoilLayout::Single`], this method basically just converts
    /// the [`Polysegment`] from [`Slot::outline`] or
    /// [`Slot::outline_winding_area`] to a [`Contour`] and wraps it in a
    /// [`Vec`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::f64::consts::PI;
    /// use approx::assert_abs_diff_eq;
    /// use stem_slot::prelude::*;
    /// use stem_slot::semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder;
    ///
    /// let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
    ///     bottom_width: Length::new::<millimeter>(10.0),
    ///     opening_width: Length::new::<millimeter>(2.0),
    ///     height: Length::new::<millimeter>(20.0),
    ///     opening_height: Length::new::<millimeter>(2.0),
    ///     angle_slot: 10.0 * PI / 180.0,
    ///     bottom_radius: Length::new::<millimeter>(2.0),
    ///     top_radius: Length::new::<millimeter>(1.0),
    ///     opening_radius: Length::new::<millimeter>(0.0),
    ///     consider_tooth_tip_leakage: true,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let contours = slot.layer_contours(&CoilLayout::Quadruple, true);
    /// assert_eq!(contours.len(), 4);
    /// assert_abs_diff_eq!(&contours[0].area(), &contours[3].area());
    /// assert_abs_diff_eq!(&contours[1].area(), &contours[2].area());
    /// ```
    fn layer_contours(&self, coil_layout: &CoilLayout, include_slot_opening: bool) -> Vec<Contour> {
        let contour = if include_slot_opening {
            self.outline().into_owned().into()
        } else {
            self.outline_winding_area().into()
        };

        match coil_layout {
            CoilLayout::Single => {
                return vec![contour];
            }
            CoilLayout::DoubleHorizontal => {
                let bb = contour.bounding_box();

                let verts_par = [[0.0, bb.ymin() - 1.0], [0.0, bb.ymax() + 1.0]];
                let vertical_line = Polysegment::from_points(verts_par.as_slice());
                let mut separated_lines =
                    contour.intersection_cut(&vertical_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

                // Check which half has positive x-values
                let bb_first = separated_lines[0].bounding_box();

                let contour_u: Polysegment; // Contour of the upper layer
                let contour_l: Polysegment; // Contour of the lower layer
                if bb_first.xmin() >= 0.0 {
                    contour_l = separated_lines.pop().unwrap();
                    contour_u = separated_lines.pop().unwrap();
                } else {
                    contour_u = separated_lines.pop().unwrap();
                    contour_l = separated_lines.pop().unwrap();
                }
                return vec![Contour::new(contour_l), Contour::new(contour_u)];
            }

            CoilLayout::DoubleVertical => {
                let bb = contour.bounding_box();

                // Separate the vertices along the y-coordinate of the contour centroid
                let center = contour.centroid();

                let verts_par = [[2.0 * bb.xmin(), center[1]], [2.0 * bb.xmax(), center[1]]];
                let horizontal_line = Polysegment::from_points(verts_par.as_slice());
                let mut separated_lines =
                    contour.intersection_cut(&horizontal_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

                // Check which half is the upper one
                let bb_first = separated_lines[0].bounding_box();

                let contour_u: Polysegment; // Contour of the upper layer
                let contour_l: Polysegment; // Contour of the lower layer
                if bb_first.center()[1] >= center[1] {
                    contour_l = separated_lines.pop().unwrap();
                    contour_u = separated_lines.pop().unwrap();
                } else {
                    contour_u = separated_lines.pop().unwrap();
                    contour_l = separated_lines.pop().unwrap();
                };
                return vec![Contour::new(contour_u), Contour::new(contour_l)];
            }
            CoilLayout::Quadruple => {
                // ==========================================================================
                // Split the path both horizontally and vertically

                // Separate the vertices along the y-coordinate of the contour centroid
                let center = contour.centroid();

                let bb = contour.bounding_box();

                let vertical_line = Polysegment::from_points(
                    [[0.0, bb.ymin() - 1.0], [0.0, bb.ymax() + 1.0]].as_slice(),
                );
                let horizontal_line = Polysegment::from_points(
                    [[2.0 * bb.xmin(), center[1]], [2.0 * bb.xmax(), center[1]]].as_slice(),
                );

                // Cut the contour vertically
                let halfes =
                    contour.intersection_cut(&vertical_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

                // Cut the halfes again horizontally
                let mut quarters: Vec<Polysegment> = Vec::with_capacity(4);
                for half in halfes {
                    let mut cutted =
                        half.intersection_cut(&horizontal_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);
                    quarters.append(&mut cutted);
                }

                let mut contour_ll: Option<Polysegment> = None; // Contour of the lower-left layer
                let mut contour_ul: Option<Polysegment> = None; // Contour of the upper-left layer
                let mut contour_lr: Option<Polysegment> = None; // Contour of the lower-right layer
                let mut contour_ur: Option<Polysegment> = None; // Contour of the upper-right layer

                let eps = std::f64::EPSILON.sqrt();

                // Identify which quarter is the upper left one, the lower left one, the upper
                // right one and the upper left one
                for quarter in quarters.into_iter() {
                    let bb = quarter.bounding_box();

                    // Check if there is a degenerated polysegment in quarters
                    if bb.width() == 0.0 || bb.height() == 0.0 {
                        continue;
                    }

                    // Check for lower-left
                    if bb.xmax() <= eps && bb.ymax() <= center[1] + eps {
                        contour_ul = Some(quarter);

                    // Check for upper-left
                    } else if bb.xmax() <= eps && bb.ymin() >= center[1] - eps {
                        contour_ll = Some(quarter);

                    // Check for lower-right
                    } else if bb.xmin() >= -eps && bb.ymax() <= center[1] + eps {
                        contour_ur = Some(quarter);

                    // Check for upper-right
                    } else if bb.xmin() >= -eps && bb.ymin() >= center[1] - eps {
                        contour_lr = Some(quarter);

                    // Quarter could not be sorted into one of the boxes. This
                    // indicates a bug.
                    } else {
                        unreachable!();
                    }
                }

                // Create contours
                let mut contours: Vec<Contour> = Vec::with_capacity(4);
                for contour in [contour_ll, contour_ul, contour_ur, contour_lr].into_iter() {
                    let mut ps = contour.expect("could not build slot shapes");

                    // Create full contour
                    let start = ps.segments().last().unwrap().stop();
                    match LineSegment::new(start, center, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                        Ok(ls) => ps.push_back(ls.into()),
                        Err(_) => (),
                    }
                    contours.push(ps.into());
                }
                return contours;
            }
            CoilLayout::MultiVertical(layers) => {
                let layers = layers.clone();
                let mut contours: Vec<Contour> = Vec::with_capacity(layers as usize);
                let mut shape_contour = contour;

                if layers > 1 {
                    let bb = shape_contour.bounding_box();
                    let slot_height = bb.height();
                    let offset = if include_slot_opening {
                        0.0
                    } else {
                        self.opening_height().get::<meter>()
                    };
                    let x_start = 2.0 * bb.xmin();
                    let x_stop = 2.0 * bb.xmax();

                    for layer in 1..layers {
                        let y = slot_height * ((layers - layer) as f64) / (layers as f64) + offset;

                        // Separate into two halfes
                        let verts_par = [[x_start, y], [x_stop, y]];
                        let horizontal_line = Polysegment::from_points(verts_par.as_slice());
                        let mut separated_lines = shape_contour.intersection_cut(
                            &horizontal_line,
                            DEFAULT_EPSILON,
                            DEFAULT_MAX_ULPS,
                        );

                        // Check which half is the upper one
                        let bb_first = separated_lines[0].bounding_box();
                        let [contour_u, contour_l] = if bb_first.center()[1] >= y {
                            let contour_l = separated_lines.pop().unwrap();
                            let contour_u = separated_lines.pop().unwrap();
                            [contour_u, contour_l]
                        } else {
                            let contour_u = separated_lines.pop().unwrap();
                            let contour_l = separated_lines.pop().unwrap();
                            [contour_u, contour_l]
                        };

                        // The upper contour is transformed into a shape and stored. The lower
                        // contour is then set as the new shape contour
                        contours.push(contour_u.into());
                        shape_contour = contour_l.into();
                    }
                }

                // Last shape
                contours.push(shape_contour);
                contours
            }
        }
    }

    /**
    Returns the contours of the winding layers as drawable objects.

    This is a wrapper around [`Slot::layer_contours`] which adds a default
    [`Style`] (with an [orange background](crate::ORANGE)) to the [`Contour`]s.
    See its docstring for an example image.
     */
    #[cfg(feature = "cairo")]
    fn drawables(
        &self,
        coil_layout: &CoilLayout,
        include_slot_opening: bool,
    ) -> Vec<DrawableCow<'_>> {
        let mut style = Style::default();
        style.background_color = crate::ORANGE;

        return self
            .layer_contours(coil_layout, include_slot_opening)
            .into_iter()
            .map(|c| DrawableCow::new(c, style.clone()))
            .collect();
    }

    /**
    Returns the self-inductance leakage coefficient of the `layer`.

    The conductors inside a slot are grouped into "layers", which are positioned
    according to the given `coil_layout`. When an AC current passes through the
    conductors of one of these layers, the resulting magnetic field acts as an
    inductance according to Lenz' rule. This so-called self-inductance can be
    calculates as:

    ```text
    Ls = μ0 * l_ax * w_sp² * lambda_s
    ```
    according to eq (3.5.13) in [1] with `μ0` being the vacuum permeability,
    `l_ax` being the axial length of the magnetic core which contains the slot
    and `w_sp²` being the number of turns in the layer.

    The self-inductance leakage coefficient `lambda_s` is given by eq. (3.5.12)
    in [1]:

    ```text
    lambda_s = int_0^h (A(x)/A)² / s(x) dx
    ```

    with `h` being the slot height, `x` being a vertical coordinate starting at
    the slot bottom, `A` being the surface area of the layer, `A(x)` being the
    area below `x` and `s(x)` being the width of the layer at `x`.

    For the full derivation, see section 3.5.2.1 of [1]. Section A.1 of [2]
    gives an example for a real slot geometry.

    Implementation-wise, this function calls
    [`Slot::mutual_inductance_leakage_coefficient`] with both `linked_layer` and
    `excitation_layer` being set to `layer`.

    # Panics
    Panics if `layer` is not smaller than the [`CoilLayout::layers`] value of
    the given `coil_layout`.

    # Literature

    >[1] Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    Maschinen, 6th edition (2008), Wiley-VCH, Weinheim
    >[2] Mathis, Stefan: Permanentmagneterregte Line-Start-Antriebe in
    Ferrittechnik, Shaker-Verlag, Düren

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

    // Single layer winding
    assert_abs_diff_eq!(slot.self_inductance_leakage_coefficient(0, &CoilLayout::Single), 1.3333, epsilon=1e-3);

    // Double-layer winding. The lower layer has a much higher self-inductance
    // than the upper, because A(x) is immediately non-zero, whereas it stays 0
    // for the first half of the layer hight in the upper layer.
    assert_abs_diff_eq!(slot.self_inductance_leakage_coefficient(0, &CoilLayout::DoubleVertical), 2.6666, epsilon=1e-3);
    assert_abs_diff_eq!(slot.self_inductance_leakage_coefficient(1, &CoilLayout::DoubleVertical), 0.6666, epsilon=1e-3);
    ```
    */
    fn self_inductance_leakage_coefficient(&self, layer: u16, coil_layout: &CoilLayout) -> f64 {
        return self.mutual_inductance_leakage_coefficient(layer, layer, coil_layout);
    }

    /**
    Returns the inductance coefficient of `linked_layer` caused by the
    `excitation_layer`.

    The conductors inside a slot are grouped into "layers", which are positioned
    according to the given `coil_layout`. When an AC current passes through the
    conductors of one of these layers, the resulting magnetic field acts as an
    inductance according to Lenz' rule both for the layer itself as well as for
    other layers in the slot. This inductance can be calculates as

    ```text
    Lo = μ0 * l_ax * w_l * w_e * lambda_o
    ```

    according to eq (3.5.22b) in [1] with `μ0` being the vacuum permeability,
    `l_ax` being the axial length of the magnetic core which contains the slot,
    `w_l` being the number of turns of the `linked_layer` and `w_e` being the
    number of turns of the `excitation_layer`. If
    `linked_layer == excitation_layer`, this simplifies to the equation shown in
    the docstring of [`Slot::self_inductance_leakage_coefficient`].

    Likewise, the inductance leakage coefficient `lambda_o` for the general case
    can be found as

    ```text
    lambda_s = int_x0^h (A_l(x)/A_l) * (A_e(x)/A_e) / s(x) dx
    ```

    with `h` being the slot height, `x` being a vertical coordinate starting at
    the slot bottom, `x0` being the lowest point of the `linked_layer` measured
    in the `x`-coordinate system, `A_l/e` being the surface area of the linked
    / excitation layer, `A_l/e(x)` being the respective area below `x` and
    `s(x)` being the width of the layer at `x`.

    From these equations, it is obvious to see that the vertical positioning of
    the layers relative to each other plays a huge role, as shown in the
    examples. See section 3.5.2.2 of [1] for more.

    # Panics
    Panics if `linked_layer` or `excitation_layer` is not smaller than the
    [`CoilLayout::layers`] value of the given `coil_layout`.

    # Literature

    >[1] Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    Maschinen, 6th edition (2008), Wiley-VCH, Weinheim

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

    // Mutual inductance of layer with itself is equal to its self-inductance
    assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 0, &CoilLayout::DoubleVertical),
        slot.self_inductance_leakage_coefficient(0, &CoilLayout::DoubleVertical),
        epsilon=1e-3
    );

    // Inductance in bottom layer caused by the top layer
    assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(0, 1, &CoilLayout::DoubleVertical),
        1.0,
        epsilon=1e-3
    );

    // Inductance in top layer caused by the bottom layer
    assert_abs_diff_eq!(
        slot.mutual_inductance_leakage_coefficient(1, 0, &CoilLayout::DoubleVertical),
        1.0,
        epsilon=1e-3
    );
    ```
     */
    fn mutual_inductance_leakage_coefficient(
        &self,
        linked_layer: u16,
        excitation_layer: u16,
        coil_layout: &CoilLayout,
    ) -> f64 {
        // Check the relationship between the layers and adjust the calculation strategy
        let slot_contour_no_opening = Contour::from(self.outline_winding_area());
        let slot_bounds_no_opening = slot_contour_no_opening.bounding_box();
        let slot_body_centroid = slot_contour_no_opening.centroid();

        let ordering = coil_layout.ordering_vertical(linked_layer, excitation_layer);
        let layer = match ordering {
            std::cmp::Ordering::Equal => {
                // Both layers are located in the same height. This equals case 1 in [1], p.
                // 316.
                linked_layer
            }
            std::cmp::Ordering::Greater => {
                // The linked layer is above the excitation layer. This equals case 2 in [1], p.
                // 316.
                linked_layer
            }
            std::cmp::Ordering::Less => {
                // The linked layer is above the excitation layer. This equals case 2 in [1], p.
                // 316.
                excitation_layer
            }
        };

        let layer_contour = &self.layer_contours(&coil_layout, false)[layer as usize];
        let layer_area = layer_contour.area();

        return inductance_leakage_coefficient(
            self,
            &slot_contour_no_opening,
            &slot_bounds_no_opening,
            layer_contour,
            &layer_bounds(
                self,
                layer,
                coil_layout,
                slot_body_centroid,
                &slot_bounds_no_opening,
                1.0,
                0.0,
            ),
            layer_area,
            &ordering,
        );
    }

    /**
    Returns the [`Slot::mutual_inductance_leakage_coefficient`] for all possible
    layer combinations for the given `coil_layout`.

    The returned matrix is square and its numbers of rows / columns equals
    [`CoilLayout::layers`] of `coil_layout`. The row contains the layer with
    the `linked_layer` where the voltage due to the leakage flux is induced,
    while the column corresponds to the `excitation_layer` carrying the current
    creating the magnetic field. This means that the diagonal contains the
    [`self_inductance_leakage_coefficient`](Slot::self_inductance_leakage_coefficient),
    while the off-diagonals contain the
    [`mutual_inductance_leakage_coefficient`](Slot::mutual_inductance_leakage_coefficient).

    This matrix does not consider either the slot opening leakage nor the tooth
    tip leakage.

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

    let coeffs = slot.leakage_coefficient_matrix(&CoilLayout::DoubleVertical);

    // Diagonals are equal to self-inductance leakage coefficient
    assert_abs_diff_eq!(
        coeffs[(0, 0)],
        slot.self_inductance_leakage_coefficient(0, &CoilLayout::DoubleVertical),
        epsilon=1e-3
    );
    assert_abs_diff_eq!(
        coeffs[(1, 1)],
        slot.self_inductance_leakage_coefficient(1, &CoilLayout::DoubleVertical),
        epsilon=1e-3
    );

    // Off-diagonals are equal to respective mutual inductance leakage coefficient.
    assert_abs_diff_eq!(
        coeffs[(0, 1)],
        slot.mutual_inductance_leakage_coefficient(0, 1, &CoilLayout::DoubleVertical),
        epsilon=1e-3
    );
    assert_abs_diff_eq!(
        coeffs[(1, 0)],
        slot.mutual_inductance_leakage_coefficient(1, 0, &CoilLayout::DoubleVertical),
        epsilon=1e-3
    );
    ```
     */
    fn leakage_coefficient_matrix(&self, coil_layout: &CoilLayout) -> DMatrix<f64> {
        let layers = coil_layout.layers();
        let dimension = layers as usize;
        let mut matrix = DMatrix::repeat(dimension, dimension, 0.0);

        /*
        Precalculate some shared values
        */
        let slot_contour_no_opening = Contour::from(self.outline_winding_area());
        let slot_bounds_no_opening = slot_contour_no_opening.bounding_box();
        let slot_body_centroid = slot_contour_no_opening.centroid();

        let all_layer_bounds: Vec<BoundingBox> = (0..layers)
            .into_par_iter()
            .map(|layer| {
                return layer_bounds(
                    self,
                    layer as u16,
                    coil_layout,
                    slot_body_centroid,
                    &slot_bounds_no_opening,
                    1.0,
                    0.0,
                );
            })
            .collect();

        let all_layer_contours = self.layer_contours(coil_layout, false);
        let all_layer_area: Vec<f64> = all_layer_contours.par_iter().map(Contour::area).collect();

        matrix
            .as_mut_slice()
            .par_iter_mut()
            .enumerate()
            .for_each(|(lin_idx, coefficient)| {
                let [excitation_layer, linked_layer] =
                    cart_lin::lin_to_cart_unchecked(lin_idx, &[dimension, dimension]);

                let ordering =
                    coil_layout.ordering_vertical(linked_layer as u16, excitation_layer as u16);
                let layer_index = match ordering {
                    std::cmp::Ordering::Equal => linked_layer,
                    std::cmp::Ordering::Less => excitation_layer,
                    std::cmp::Ordering::Greater => linked_layer,
                };

                *coefficient = inductance_leakage_coefficient(
                    self,
                    &slot_contour_no_opening,
                    &slot_bounds_no_opening,
                    &all_layer_contours[layer_index],
                    &all_layer_bounds[layer_index],
                    all_layer_area[layer_index],
                    &ordering,
                );
            });

        return matrix;
    }

    /// Returns the tooth tip leakage coefficient as a function of the magnetic
    /// / effective air gap.
    ///
    /// The tooth tip leakage flux is the part of the magnetic flux which exits
    /// the tooth tip, but does not cross over the air gap and instead takes
    /// an arc path back to the neighboring tooth tip.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Slot leakage flux overview][slot_leakage_flux_overview]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "slot_leakage_flux_overview",
            "docs/img/slot_leakage_flux_overview.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// This flux is heavily influenced by the [`Slot::opening_width`] in
    /// between the teeth and the magnetic air gap. Generally speaking, a
    /// smaller slot opening increases this flux, while a smaller
    /// `magnetic_air_gap` decreases it. For an in-depth description of the
    /// phenomen, see e.g.
    /// > Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    /// Maschinen, 6th edition (2008), Wiley-VCH, Weinheim (section 3.7.1)
    ///
    /// This method returns a dimensionless factor. To obtain the actual leakage
    /// inductance, multiply that factor with the main inductance of the winding
    /// in the slot. The leakage flux for a particular current can then be found
    /// by multiplying the leakage inductance with that current.
    ///
    /// The default implementation of this method uses the free function
    /// [`leakage_coefficient_tooth_tip`], see its docstring for details. This
    /// separation between interface (this method) and implementation allows
    /// using the underlying function as part of a custom implementation.
    /// For an example of this pattern, see the source code of the [`Slot`]
    /// implementation for
    /// [`RectangularSlot`](crate::rectangular::RectangularSlot).
    fn leakage_coefficient_tooth_tip(&self, magnetic_air_gap: Length) -> f64 {
        leakage_coefficient_tooth_tip(self.opening_width(), magnetic_air_gap)
    }

    /// Returns the slot opening leakage coefficient.
    ///
    /// A part of the magnetic flux created by the coil(s) inside the slot
    /// closes over the slot opening (see image below). This flux is calculated
    /// by multiplying the slot opening leakage inductance with the current
    /// going through the coil(s). The slot opening leakage inductance itself
    /// is the product of the main winding inductance and the slot opening
    /// factor which is provided by this method.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Slot leakage flux overview][slot_leakage_flux_overview]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "slot_leakage_flux_overview",
            "docs/img/slot_leakage_flux_overview.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// The default implementation of the method assumes that the slot opening
    /// is parallel-sided. In that case, the coefficient becomes the quotient
    /// `opening_height / opening_width`, see eq. (3.7.1f) in [1]. Even if the
    /// slot opening is not parallel sided, it is usually sufficient to
    /// approximate it as such by using a mean value for the opening width (see
    /// [1], p. 325). In case the slot is closed, this method simply returns
    /// zero.
    ///
    /// >[1]: Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    /// Maschinen, 6th edition (2008), Wiley-VCH, Weinheim
    ///
    /// # Examples
    ///
    /// ```
    /// use stem_slot::prelude::*;
    ///
    /// let slot = RectangularSlot::new(
    ///     Length::new::<millimeter>(10.0),
    ///     Length::new::<millimeter>(2.0),
    ///     Length::new::<millimeter>(20.0),
    ///     Length::new::<millimeter>(1.0),
    ///     true,
    /// ).expect("valid inputs");
    /// assert_eq!(slot.leakage_coefficient_opening(), 0.5); // height (1) / width (2)
    /// ```
    fn leakage_coefficient_opening(&self) -> f64 {
        if self.opening_width().get::<meter>() > 0.0 {
            return f64::from(self.opening_height() / self.opening_width());
        } else {
            return 0.0;
        }
    }

    /// Returns a [`CurrentDisplacementCalculator`] which can be used to
    /// calculate the
    /// [`CurrentDisplacementCoefficients`](crate::current_displacement::CurrentDisplacementCoefficients).
    ///
    /// In massive conductors, an alternating current is not evenly spread
    /// across the cross-section, but instead is "displaced" by its own magnetic
    /// field. This displacement reduces the effective cross section of the
    /// conductor, resulting in an increased resistance and reduced inductance.
    /// The effect depends on the slot / conductor geometry as well as on
    /// external factors like the frequency of the alternating current, the
    /// electric conductivity and the relative permeability of the conductor.
    ///
    /// This method returns a [`CurrentDisplacementCalculator`] which allows the
    /// efficient calculation of the
    /// [`CurrentDisplacementCoefficients`](crate::current_displacement::CurrentDisplacementCoefficients)
    /// for the slot geometry of `self`. The slot surface is separated into
    /// multiple/ rectangular [`slices`](Slot::slices) and the coefficients are
    /// calculated piece-wise. For more information, see
    /// >Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
    /// Maschinen, 6th edition (2008), Wiley-VCH, Weinheim (section 5.3)
    ///
    /// The minimum number of slices is specified by `min_num_slices`, see the
    /// docstring of [`Slot::slices`]. Generally speaking, the higher this
    /// number, the more precise and expensive the calculation. In practice, a
    /// value of 50 delivers sufficient results even for complex geometries.
    ///
    /// The following graph shows a comparison for the special case of an open
    /// rectangular open slot, where an analytic solution exists (see
    /// [`CurrentDisplacementCoefficients::from_rectangular_open_slot`](crate::current_displacement::CurrentDisplacementCoefficients::from_rectangular_open_slot)).
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
    ///
    /// # Examples
    /// ```
    /// use approx::assert_abs_diff_eq;
    /// use stem_slot::prelude::*;
    ///
    /// let slot = RectangularSlot::new(
    ///     Length::new::<millimeter>(10.0),
    ///     Length::new::<millimeter>(5.0),
    ///     Length::new::<millimeter>(20.0),
    ///     Length::new::<millimeter>(0.0),
    ///     true,
    /// ).expect("valid inputs");
    ///
    /// let el_conductivity = ElectricalConductivity::new::<siemens_per_meter>(37.0 * 1e6);
    /// let rel_permeability = 1.0;
    ///
    /// // Reuse of calculator for different frequencies
    /// let mut calc = slot.current_displacement_coefficients(50);
    /// assert_abs_diff_eq!(
    ///     calc.eval(Frequency::new::<hertz>(50.0), el_conductivity, rel_permeability).resistance,
    ///     1.5757,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     calc.eval(Frequency::new::<hertz>(100.0), el_conductivity, rel_permeability).resistance,
    ///     2.381,
    ///     epsilon = 1e-3
    /// );
    ///
    /// // Higher number of slices
    /// let calc_hi_prec = slot.current_displacement_coefficients(100);
    /// assert_abs_diff_eq!(
    ///     calc.eval(Frequency::new::<hertz>(50.0), el_conductivity, rel_permeability).resistance,
    ///     1.5757,
    ///     epsilon = 1e-3
    /// );
    ///
    /// // Comparison with analytical solution
    /// assert_abs_diff_eq!(
    ///     CurrentDisplacementCoefficients::from_rectangular_open_slot(
    ///         slot.height(),
    ///         Frequency::new::<hertz>(50.0),
    ///         el_conductivity,
    ///         rel_permeability
    ///     ).resistance,
    ///     1.5757,
    ///     epsilon = 1e-3
    /// );
    /// ```
    fn current_displacement_coefficients(
        &self,
        min_num_slices: usize,
    ) -> CurrentDisplacementCalculator {
        return CurrentDisplacementCalculator::new(self, min_num_slices);
    }

    /// Separates the slot in horizontal slices and returns their bounding
    /// boxes, starting at the slot bottom.
    ///
    /// This method is used by [`Slot::current_displacement_coefficients`] to
    /// approximate the slot area by multiple stacked rectangles. The
    /// `min_num_slices` defines the maximum height of a single rectangle as
    /// `self.height() / min_num_slices`. As the name suggests, the actual
    /// number of generated slices can be (much) higher, because e.g. arc
    /// segments are again split into partial arcs covering at most 10 degree.
    /// Therefore, this value is a lower limit on the desired precision.
    #[doc = ""]
    #[cfg_attr(feature = "doc-images", doc = "![Slices comparison][slices_comp]")]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image("slices_comp", "docs/img/slices_comp.svg")
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// As described in the docstring of [`CurrentDisplacementCalculator`], the
    /// [`CurrentDisplacementCoefficients`](crate::current_displacement::CurrentDisplacementCoefficients)
    /// of a conductor filling an arbitrary slot geometry can be found by
    /// separating the slot area in multiple parallel conductors and calculating
    /// the currents through each one. This method delivers the dimensions of
    /// each rectangular conductor.
    ///
    /// The default implementation makes the following assumptions:
    /// - The slot outline is symmetrical about the y-axis. This implies that
    /// the slot outline crosses the y-axis exactly once at the slot bottom.
    /// - Drawing a horizontal line anywhere through the slot does not result
    /// in more than two intersections
    ///
    /// The image below shows two examples where these assumptions are not
    /// fulfilled. In such a case, this method must be overwritten. The order
    /// of the returned [`BoundingBox`]es must be slot-bottom-to-opening.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Assumptions are not fulfilled][non_conform_slices]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image("non_conform_slices", "docs/img/non_conform_slices.svg")
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// # Examples
    ///
    /// ```
    /// use approx::assert_abs_diff_eq;
    /// use stem_slot::prelude::*;
    ///
    /// let slot = RectangularSlot::new(
    ///     Length::new::<millimeter>(10.0),
    ///     Length::new::<millimeter>(5.0),
    ///     Length::new::<millimeter>(20.0),
    ///     Length::new::<millimeter>(2.0),
    ///     true,
    /// ).expect("valid inputs");
    ///
    /// let slices = slot.slices(10);
    /// assert_eq!(slices.len(), 10);
    ///
    /// // Assert that the area covered by all slices is equivalent to that of the slot
    /// let area: f64 = slices.iter().map(|b|b.height() * b.width()).sum();
    /// assert_abs_diff_eq!(area, slot.area().get::<square_meter>(), epsilon=1e-6);
    /// ```
    fn slices(&self, min_num_slices: usize) -> Vec<BoundingBox> {
        let binding = self.outline();
        let mut point_iter = binding.polygonize(Polygonizer::PerType {
            arc: SegmentPolygonizer::MaximumAngle(TAU / 36.0),
            straight: SegmentPolygonizer::InnerSegments(1),
        });

        let max_slice_height = self.height().get::<meter>() / (min_num_slices as f64);

        // Middle of the vertical right side
        let mut bbs: Vec<BoundingBox> = Vec::with_capacity(min_num_slices);

        /*
        Iterate through the polygon points until the sign changes. This
        indicates that the slot bottom has been reached (even if it is an arc,
        because that one has been polygonized as well). After that, start
        building the vector of bounding boxes from the slot bottom up to the
        air gap
        */
        if let Some(mut pt1) = point_iter.next() {
            let initial_sign = pt1[0].signum();
            for pt2 in point_iter {
                // Skip all points with the same x-sign as the initial point
                // (slot bottom hasn't been reached yet)
                if pt1[0].signum() == initial_sign {
                    pt1 = pt2;
                    continue;
                }

                // Skip sections which have a very small incline
                let delta_x = pt2[0] - pt1[0];
                let delta_y = (pt2[1] - pt1[1]).abs();
                if ulps_eq!(
                    delta_y,
                    0.0,
                    epsilon = DEFAULT_EPSILON,
                    max_ulps = DEFAULT_MAX_ULPS
                ) {
                    pt1 = pt2;
                    continue;
                }

                /*
                Divide the section in slices of even height, the maximum height being
                limited by max_slice_height
                */
                let n_slices_section = (delta_y / max_slice_height).ceil();
                let slice_height = delta_y / n_slices_section;

                for ii in 0..(n_slices_section as usize) {
                    let d = (n_slices_section - ii as f64) - 0.5;
                    let x = pt2[0].abs() + d * delta_x / n_slices_section;
                    let y_middle = pt2[1] + d * slice_height;
                    bbs.push(BoundingBox::new(
                        -x,
                        x,
                        y_middle - 0.5 * slice_height,
                        y_middle + 0.5 * slice_height,
                    ));
                }

                // Prepare the next iteration
                pt1 = pt2;
            }
        }

        return bbs;
    }
}

dyn_clone::clone_trait_object!(Slot);

/**
An iterator returning all parts of an outline bordering a layer

This struct is created by [`Slot::layer_outlines`].
 */
#[derive(Clone, Debug)]
pub struct LayerOutlines {
    inner: std::vec::IntoIter<Polysegment>,
    layer_bounds: BoundingBox,
}

impl LayerOutlines {
    /// Returns the total length of all outlines.
    pub fn length(self) -> Length {
        Length::new::<meter>(self.into_iter().map(|ps| ps.length()).sum::<f64>())
    }
}

impl Iterator for LayerOutlines {
    type Item = Polysegment;

    fn next(&mut self) -> Option<Self::Item> {
        let ps = self.inner.next()?;
        if self
            .layer_bounds
            .approx_covers(&ps.bounding_box(), DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
        {
            return Some(ps);
        }
        return self.next();
    }
}

lazy_static::lazy_static! {
    static ref LEAKAGE_COEFFICIENT_TOOTH_TIP: AkimaSpline = {
         // Interpolation from 3.7.2 of [MVP08] (values read out by hand!)
        let x = vec![
            0.125, 0.275, 0.5, 0.9, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0,
        ];
        let y = vec![
            1.0, 0.82, 0.62, 0.42, 0.38, 0.24, 0.13, 0.05, 0.0, -0.05, -0.085, -0.11, -0.13, -0.148,
            -0.159, -0.17,
        ];
        let len = y.len() - 1;
        let ml = vec![(y[0] - y[1]) / (x[0] - x[1])];
        let mr = vec![(y[len - 1] - y[len]) / (x[len - 1] - x[len])];
        AkimaSpline::new(x, y, Some(ml), Some(mr))
            .expect("spline can be constructed from given data")
    };
}

/// Returns the tooth tip leakage coefficient as a function of the magnetic
/// / effective air gap.
///
/// For a general introduction to the tooth tip leakage coefficient, see the
/// docstring of [`Slot::leakage_coefficient_tooth_tip`]. This function serves
/// as the default implementation of the method and uses the heuristic graph
/// 3.7.2 of
/// > Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
/// Maschinen, 6th edition (2008), Wiley-VCH, Weinheim (section 3.7.1)
///
/// The image below shows the resulting coefficient as a function of the
/// ratio `opening_width / magnetic_air_gap`.
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Tooth tip leakage flux graph][leakage_coefficient_tooth_tip]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "leakage_coefficient_tooth_tip",
        "docs/img/leakage_coefficient_tooth_tip.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/// In this approximation, the coefficient becomes negative for large ratios
/// `opening_width / magnetic_air_gap`as some of the slot opening leakage flux
/// gets "pulled out" of the opening and instead crosses
/// the air gap, leading to a net reduction of the overall leakage flux. This is
/// due to the fact that the path crossing the air gap (twice) starts to have a
/// lower magnetic resistance than the path accross the slot opening. Since
/// the analytic slot opening flux calculation does not factor this in, the
/// negative tooth tip leakage flux is used as a "compensation".
///
/// # Examples
///
/// ```
/// use approx::assert_abs_diff_eq;
/// use stem_slot::prelude::*;
/// use stem_slot::slot::leakage_coefficient_tooth_tip;
///
/// let ow = Length::new::<millimeter>(2.0);
///
/// // Magnetic path for crossing the air gap twice roughly equivalent to slot opening width
/// let ag_a = Length::new::<millimeter>(1.0);
/// assert_abs_diff_eq!(leakage_coefficient_tooth_tip(ow, ag_a), 0.13, epsilon=1e-3);
///
/// // Magnetic path for crossing the air gap twice much smaller than slot opening width
/// let ag_b = Length::new::<millimeter>(0.5);
/// assert_abs_diff_eq!(leakage_coefficient_tooth_tip(ow, ag_b), -0.05, epsilon=1e-3);
/// ```
pub fn leakage_coefficient_tooth_tip(opening_width: Length, magnetic_air_gap: Length) -> f64 {
    LEAKAGE_COEFFICIENT_TOOTH_TIP
        .eval(f64::from(opening_width / magnetic_air_gap))
        .unwrap_or(0.0)
}

/// Calculates the second side length of a semi-regular polygon from a
/// `given_side_length`, the `radius` of the smallest circle containing the
/// polygon and the `number_of_sides`.
///
/// A semi-regular polygon is a polygon which has `number_of_sides` sides,
/// where `number_of_sides/2` sides have the `given_side_length` and
/// `number_of_sides/2` have the length returned by this function. The two side
/// lengths are alternating along the polygon surface. The following image shows
/// an example together with the formulae used to calculate the return value
/// (which are derived from the commonly known circular segment relations, see
/// e.g. here: <https://en.wikipedia.org/wiki/Circular_segment>):
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Semi-regular polygon][semi_regular_polygon]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("semi_regular_polygon", "docs/img/semi_regular_polygon.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
///
/// This method returns `None` if `number_of_sides` is odd or `first_side` /
/// `outer_radius` are not positive.
///
/// The main purpose of this method is to derive the slot widths for a rotary
/// core where the tooth width is fixed. In this case, `given_side_length` is
/// the tooth width, `radius` is the air gap radius and `number_of_sides` is two
/// times the number of teeth.
///
/// Examples
///
/// ```
/// use approx::assert_abs_diff_eq;
/// use stem_slot::slot::semi_regular_polygon_side_length;
///
/// let second_side = semi_regular_polygon_side_length(1.0, 2.0, 12).unwrap();
/// assert_abs_diff_eq!(1.070466, second_side, epsilon = 1e-6);
/// ```
pub fn semi_regular_polygon_side_length(
    given_side_length: f64,
    radius: f64,
    number_of_sides: usize,
) -> Option<f64> {
    use num::Integer;
    if number_of_sides.is_odd() || given_side_length < 0.0 || radius < 0.0 {
        return None;
    }

    let angle_given_side = 2.0 * (given_side_length / (2.0 * radius)).asin();
    let angle_searched_side = TAU / (number_of_sides as f64 / 2.0) - angle_given_side;
    return Some(2.0 * radius * (angle_searched_side / 2.0).sin());
}

/// Returns the slot width at the given slot height.
///
/// This is `s(x)` in the formulae given in
/// [`Slot::self_inductance_leakage_coefficient`] and
/// [`Slot::mutual_inductance_leakage_coefficient`]
fn width<S: Slot + ?Sized>(
    slot: &S,
    vertical_slot_coord: Length,
    contour: &Contour,
    slot_bounds: &BoundingBox,
) -> Length {
    // Case x = 0: width equals slot opening
    if vertical_slot_coord == Length::new::<meter>(0.0) {
        return slot.opening_width();
    }

    let vertices = vec![
        [2.0 * slot_bounds.xmin(), vertical_slot_coord.get::<meter>()],
        [2.0 * slot_bounds.xmax(), vertical_slot_coord.get::<meter>()],
    ];
    let parallel_line = Polysegment::from_points(&vertices);
    let intersections =
        contour.intersections_par(&parallel_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

    // One or no intersection -> Secant length is zero
    if intersections.len() < 2 {
        return Length::new::<meter>(0.0);
    } else {
        // Identify the intersections with the largest positive or negative x-value
        let mut inter_pos = intersections[0];
        let mut inter_neg = intersections[0];

        for intersection in intersections.iter().skip(1) {
            if intersection.point[0] > inter_pos.point[0] {
                inter_pos = *intersection;
            }
            if intersection.point[0] < inter_neg.point[0] {
                inter_neg = *intersection;
            }
        }

        return Length::new::<meter>(inter_pos.point[0] - inter_neg.point[0]);
    }
}

fn layer_bounds<S: Slot + ?Sized>(
    slot: &S,
    layer: u16,
    coil_layout: &CoilLayout,
    slot_body_centroid: [f64; 2],
    slot_bounds_no_opening: &BoundingBox,
    x_offset: f64,
    y_offset: f64,
) -> BoundingBox {
    match coil_layout {
        CoilLayout::Single => {
            return BoundingBox::new(
                slot_bounds_no_opening.xmin() - x_offset,
                slot_bounds_no_opening.xmax() + x_offset,
                slot.opening_height().get::<meter>(),
                slot.height().get::<meter>(),
            );
        }
        CoilLayout::DoubleVertical => {
            if layer == 0 {
                // First layer is at the slot bottom => See documentation of CoilLayout
                return BoundingBox::new(
                    slot_bounds_no_opening.xmin() - x_offset,
                    slot_bounds_no_opening.xmax() + x_offset,
                    slot_body_centroid[1],
                    slot.height().get::<meter>() + y_offset,
                );
            } else {
                // Second layer is at the slot top => See documentation of CoilLayout
                return BoundingBox::new(
                    slot_bounds_no_opening.xmin() - x_offset,
                    slot_bounds_no_opening.xmax() + x_offset,
                    slot.opening_height().get::<meter>() - y_offset,
                    slot_body_centroid[1],
                );
            }
        }
        CoilLayout::DoubleHorizontal => {
            // Tooth coil arrangement => Two coils aranged horizontally next to each other
            if layer == 0 {
                return BoundingBox::new(
                    slot_bounds_no_opening.xmin() - x_offset,
                    0.0,
                    slot.opening_height().get::<meter>() - y_offset,
                    slot.height().get::<meter>() + y_offset,
                );
            } else {
                return BoundingBox::new(
                    0.0,
                    slot_bounds_no_opening.xmax() + x_offset,
                    slot.opening_height().get::<meter>() - y_offset,
                    slot.height().get::<meter>() + y_offset,
                );
            }
        }
        CoilLayout::Quadruple => {
            // Essentially the same code as for DoubleVertical, but layer 0 and 1 are at the
            // bottom and layer 2 and 3 are at the top
            match layer {
                0 => {
                    return BoundingBox::new(
                        slot_bounds_no_opening.xmin() - x_offset,
                        0.0,
                        slot.opening_height().get::<meter>() - y_offset,
                        slot_body_centroid[1],
                    );
                }
                1 => {
                    return BoundingBox::new(
                        0.0,
                        slot_bounds_no_opening.xmax() + x_offset,
                        slot.opening_height().get::<meter>() - y_offset,
                        slot_body_centroid[1],
                    );
                }
                2 => {
                    return BoundingBox::new(
                        slot_bounds_no_opening.xmin() - x_offset,
                        0.0,
                        slot_body_centroid[1],
                        slot.height().get::<meter>() + y_offset,
                    );
                }
                3 => {
                    return BoundingBox::new(
                        0.0,
                        slot_bounds_no_opening.xmax() + x_offset,
                        slot_body_centroid[1],
                        slot.height().get::<meter>() + y_offset,
                    );
                }
                _ => unreachable!(),
            }
        }
        CoilLayout::MultiVertical(layers) => {
            let delta_height =
                (slot.height() - slot.opening_height()).get::<meter>() / *layers as f64;

            let [mult_min, mult_max] = if layer == 0 {
                [0.0, 1.0]
            } else if layer + 1 == *layers {
                [1.0, 0.0]
            } else {
                [0.0, 0.0]
            };

            return BoundingBox::new(
                slot_bounds_no_opening.xmin() - x_offset,
                slot_bounds_no_opening.xmax() + x_offset,
                (*layers - layer - 1) as f64 * delta_height + slot.opening_height().get::<meter>()
                    - mult_min * y_offset,
                (*layers - layer) as f64 * delta_height
                    + slot.opening_height().get::<meter>()
                    + mult_max * y_offset,
            );
        }
    }
}

/// Internal function which is not meant to be called directly.
fn inductance_leakage_coefficient<S: Slot + ?Sized>(
    slot: &S,
    slot_contour: &Contour,
    slot_bounds: &BoundingBox,
    linked_layer_contour: &Contour,
    linked_layer_bounds: &BoundingBox,
    linked_layer_area: f64,
    ordering_linked_to_excitation_layer: &std::cmp::Ordering,
) -> f64 {
    // Theta(x) is a squared function of the area ratio (we are located on the
    // height of both linked and excitation layer)
    let integrand_exc_squared = |vertical_coord: f64| {
        let width = width(
            slot,
            Length::new::<meter>(vertical_coord),
            slot_contour,
            slot_bounds,
        );
        if width.get::<meter>() <= 0.0 {
            return 0.0;
        }
        if vertical_coord <= linked_layer_bounds.ymin() {
            // 1/s => area above the layer
            return 1.0 / width.get::<meter>();
        } else {
            // (Delta A / A)^2 /s => area in the layer
            let delta_area = lower_part_of_layer_area(
                vertical_coord,
                &linked_layer_contour,
                &linked_layer_bounds,
            );
            return (f64::from(delta_area / linked_layer_area)).powi(2) / width.get::<meter>();
        }
    };

    // Theta(x) is linear rising (we are located in the excitation layer, the linked
    // layer is above or below)
    let integrand_exc_lin = |vertical_coord: f64| {
        let width = width(
            slot,
            Length::new::<meter>(vertical_coord),
            slot_contour,
            slot_bounds,
        );
        if width.get::<meter>() <= 0.0 {
            return 0.0;
        }
        // Delta A / A /s => area in the layer ==> eq. 3.5.25 in [MVP08]
        let delta_area =
            lower_part_of_layer_area(vertical_coord, &linked_layer_contour, &linked_layer_bounds);
        return f64::from(delta_area / linked_layer_area) / width.get::<meter>();
    };

    // Theta(x) is constant (we are located above the excitation layer)
    let integrand_exc_const = |vertical_coord: f64| {
        let width = width(
            slot,
            Length::new::<meter>(vertical_coord),
            slot_contour,
            slot_bounds,
        );
        if width.get::<meter>() <= 0.0 {
            return 0.0;
        }
        // 1/s => area above the layer
        return 1.0 / width.get::<meter>();
    };

    // Initialize the quadrature rule
    let quad = gauss_quad::GaussLegendre::init(16); // polynomial degree 16 was determined empirically

    /*
    The parts of the integration function are separated to avoid numerical errors.
    Since the quad operation works with polynomial approximations, a discontinuity like it occurs between
    the linked layer border and the upper part of the slot leads to numerical errors. Those errors disappear
    if the integration function is separated into continuous sections.
    */
    match ordering_linked_to_excitation_layer {
        std::cmp::Ordering::Equal => {
            // Case 1 in [MVP08], p. 316
            return quad.integrate(
                linked_layer_bounds.ymin(),
                linked_layer_bounds.ymax(),
                integrand_exc_squared,
            ) + quad.integrate(
                slot.opening_height().get::<meter>(),
                linked_layer_bounds.ymin(),
                integrand_exc_const,
            );
        }
        _ => {
            // Case 2 in [MVP08], p. 316
            return quad.integrate(
                linked_layer_bounds.ymin(),
                linked_layer_bounds.ymax(),
                integrand_exc_lin,
            ) + quad.integrate(
                slot.opening_height().get::<meter>(),
                linked_layer_bounds.ymin(),
                integrand_exc_const,
            );
        }
    }
}

/**
Returns the area of the selected layer as a function of the vertical slot
coordinate, which starts at the slot bottom.

# Panics
Panics if the given coil index is larger than the total number of coils in the
coil layout.
*/
fn lower_part_of_layer_area(
    vertical_slot_coord: f64,
    layer_contour: &Contour,
    layer_bounds: &BoundingBox,
) -> f64 {
    if vertical_slot_coord >= layer_bounds.ymax() {
        return 0.0;
    }

    let lb_adjusted = if vertical_slot_coord > layer_bounds.ymin() {
        BoundingBox::new(
            layer_bounds.xmin(),
            layer_bounds.xmax(),
            vertical_slot_coord,
            layer_bounds.ymax(),
        )
    } else {
        return layer_contour.area();
    };
    let clb = Contour::from(lb_adjusted.clone());

    return layer_contour
        .intersection_cut(clb.polysegment(), DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
        .into_iter()
        .filter(|ps| {
            lb_adjusted.approx_covers(&ps.bounding_box(), DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
        })
        .reduce(|mut ps1, mut ps2| {
            ps1.append(&mut ps2);
            ps1
        })
        .map(|c| Contour::from(c).area())
        .unwrap_or(0.0);
}

pub(crate) fn rotating_core_slot_y_offset(
    air_gap_radius: Length,
    slot_opening_width: Length,
) -> Length {
    use uom::typenum::P2;
    return air_gap_radius
        - 0.5 * (4.0 * air_gap_radius.powi(P2::new()) - slot_opening_width.powi(P2::new())).sqrt();
}

pub(crate) fn slot_side_bottom_and_top_width_from_rot_core(
    tooth_width: Length,
    air_gap_radius: Length,
    yoke_radius: Length,
    slots: u16,
    side_height: Length,
    opening_width: Length,
    opening_height: Length,
) -> [Length; 2] {
    use uom::typenum::P2;
    let y_offset = rotating_core_slot_y_offset(air_gap_radius, opening_width);

    let angle_slot = if air_gap_radius < yoke_radius {
        TAU / slots as f64
    } else {
        -TAU / slots as f64
    };

    // Calculate the slot width at bottom and top
    let r_slot_opening_top = ((air_gap_radius + opening_height - y_offset).powi(P2::new())
        + (0.5 * opening_width).powi(P2::new()))
    .sqrt(); // Radius of the slot opening top edge
    let b_tooth_tip_top = Length::new::<meter>(
        semi_regular_polygon_side_length(
            opening_width.get::<meter>(),
            r_slot_opening_top.get::<meter>(),
            2 * slots as usize,
        )
        .unwrap(),
    );

    let delta_b_tooth = 0.5 * (b_tooth_tip_top - tooth_width);
    let x = delta_b_tooth / (0.5 * angle_slot).cos();
    let b_top = 2.0 * x + opening_width;
    let b_bottom = b_top + 2.0 * side_height * (0.5 * angle_slot).tan();

    return [b_bottom, b_top];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rectangular::RectangularSlot;
    use approx;

    #[test]
    fn test_lower_part_of_layer_area_rectangular() {
        let opening_height = Length::new::<millimeter>(1.0);
        let opening_width = Length::new::<millimeter>(3.0);
        let width = Length::new::<millimeter>(3.0);
        let height = Length::new::<millimeter>(20.0);
        let slot =
            RectangularSlot::new(width, opening_width, height, opening_height, true).unwrap();

        let slot_contour = Contour::new(slot.outline_winding_area());

        // Single layer
        let bounds = layer_bounds(
            &slot,
            0,
            &CoilLayout::Single,
            slot_contour.centroid(),
            &slot_contour.bounding_box(),
            1.0,
            0.0,
        );

        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(0.0, &slot_contour, &bounds),
            ((height - opening_height) * width).get::<square_meter>(),
            epsilon = 1e-6
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(10e-3, &slot_contour, &bounds),
            10e-3 * width.get::<meter>(),
            epsilon = 1e-6
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(5e-3, &slot_contour, &bounds),
            15e-3 * width.get::<meter>(),
            epsilon = 1e-6
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(20e-3, &slot_contour, &bounds),
            0.0,
            epsilon = 1e-6
        );

        // Double layer horizontal
        let bounds = layer_bounds(
            &slot,
            0,
            &CoilLayout::DoubleHorizontal,
            slot_contour.centroid(),
            &slot_contour.bounding_box(),
            1.0,
            0.0,
        );

        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(10e-3, &slot_contour, &bounds),
            7.5e-6,
            epsilon = 1e-6
        );

        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(12e-3, &slot_contour, &bounds),
            6e-6,
            epsilon = 1e-6
        );

        // Double layer vertical
        let bounds = layer_bounds(
            &slot,
            0,
            &CoilLayout::DoubleVertical,
            slot_contour.centroid(),
            &slot_contour.bounding_box(),
            1.0,
            0.0,
        );

        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(15e-3, &slot_contour, &bounds),
            5e-3 * width.get::<meter>(),
            epsilon = 1e-8
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(10e-3, &slot_contour, &bounds),
            5.7e-5,
            epsilon = 1e-8
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(5e-3, &slot_contour, &bounds),
            5.7e-5,
            epsilon = 1e-8
        );

        let bounds = layer_bounds(
            &slot,
            1,
            &CoilLayout::DoubleVertical,
            slot_contour.centroid(),
            &slot_contour.bounding_box(),
            1.0,
            0.0,
        );

        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(9e-3, &slot_contour, &bounds),
            1.5e-3 * width.get::<meter>(),
            epsilon = 1e-6
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(3e-3, &slot_contour, &bounds),
            (9.5e-3 - 2e-3) * width.get::<meter>(),
            epsilon = 1e-6
        );
        approx::assert_abs_diff_eq!(
            lower_part_of_layer_area(10.5e-3, &slot_contour, &bounds),
            0.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_slot_side_bottom_and_top_width_from_rot_core() {
        // Values from [Mat19] slot
        let tooth_width = Length::new::<millimeter>(3.415);
        let air_gap_radius = Length::new::<millimeter>(55.0);
        let yoke_radius = Length::new::<millimeter>(85.0);
        let slots = 36;
        let side_height = Length::new::<millimeter>(17.0);
        let opening_height = Length::new::<millimeter>(0.75);
        let opening_width = Length::new::<millimeter>(2.0);

        let [b_bottom, b_top] = slot_side_bottom_and_top_width_from_rot_core(
            tooth_width,
            air_gap_radius,
            yoke_radius,
            slots,
            side_height,
            opening_width,
            opening_height,
        );

        approx::assert_abs_diff_eq!(b_bottom.get::<millimeter>(), 9.29996, epsilon = 1e-3);
        approx::assert_abs_diff_eq!(b_top.get::<millimeter>(), 6.32535, epsilon = 1e-3);
    }
}
