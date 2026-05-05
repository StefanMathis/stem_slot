use akima_spline::AkimaSpline;
use approx::{ulps_eq, ulps_ne};
use dyn_clone::DynClone;
use gauss_quad;
use nalgebra::DMatrix;
use planar_geo::prelude::*;
use rayon::prelude::*;
use std::any::Any;
use std::borrow::Cow;
use std::f64::consts::{FRAC_PI_2, TAU};
use stem_material::prelude::*;

#[cfg(feature = "serde")]
use dyn_quantity::{deserialize_angle, deserialize_quantity};

#[cfg(feature = "serde")]
use serde::Deserialize;

use crate::coil_layout::CoilLayout;
use crate::current_displacement::CurrentDisplacementCoefficients;
use crate::current_displacement::current_displacement_coefficients_numeric;

/**
The `Slot` trait is used to implement slot objects. The trait implements a number of default methods
(e.g. for the calculation of the slot leakage factor), which can be overwritten in a specific implementation.
Some other methods however need to be implemented by each individual slot (e.q. the function representing the slot contour).
*/
#[cfg_attr(feature = "serde", typetag::serde)]
pub trait Slot: Send + Sync + std::fmt::Debug + DynClone + Any + 'static {
    /// Returns the total slot height.
    fn height(&self) -> Length;

    /// Returns the slot opening width
    fn opening_width(&self) -> Length;

    /// Returns the slot opening width
    fn opening_height(&self) -> Length;

    /// Returns the effective magnetic slot opening height.
    fn magnetic_opening_height(&self) -> Length;

    /// If true, the tooth tip leakage is considered when calculating the
    /// leakage coefficent
    fn consider_tooth_tip_leakage(&self) -> bool;

    /// Returns the polysegment forming the slot contour. If the slot is open,
    /// the polysegment is open as well.
    fn polysegment(&self) -> Cow<'_, Polysegment>;

    // Default implementations
    // =======================================================================================================================

    /// Returns the slot contour.
    fn contour(&self) -> Contour {
        return self.polysegment().into_owned().into();
    }

    /// Returns the segment chain without slot opening
    fn polysegment_main_body(&self) -> Polysegment {
        let contour = self.contour();
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
            .expect("at least one segment must be higher than the opening height");
    }

    fn contour_main_body(&self) -> Contour {
        return self.polysegment_main_body().into();
    }

    /// Return the slot outline (slot shape line minus the slot opening, if the
    /// slot is open)
    fn outline(&self) -> Length {
        return Length::new::<meter>(self.polysegment().length());
    }

    /// Return the part of the slot outline bordering the selected layer
    fn layer_outline(&self, layer: u16, coil_layout: &CoilLayout) -> Length {
        let polysegment = self.polysegment_main_body();
        let centroid = Contour::new(polysegment.clone()).centroid();
        let layer_bounds = layer_bounds_priv(
            self,
            layer,
            coil_layout,
            centroid,
            &polysegment.bounding_box(),
            1.0,
            1.0,
        );

        // Sum up all parts of the segment chain which are within bounds
        let sum = polysegment
            .intersection_cut(
                &Polysegment::from(&layer_bounds),
                DEFAULT_EPSILON,
                DEFAULT_MAX_ULPS,
            )
            .into_iter()
            .filter_map(|chain| {
                if layer_bounds.approx_covers(
                    &chain.bounding_box(),
                    DEFAULT_EPSILON,
                    DEFAULT_MAX_ULPS,
                ) {
                    Some(chain.length())
                } else {
                    None
                }
            })
            .sum();

        return Length::new::<meter>(sum);
    }

    /// Return true if the slot is open (to the air gap) and false if not.
    fn is_open(&self) -> bool {
        return self.opening_width().get::<meter>() > 0.0;
    }

    /**
    Calculate the self inductance leakage coefficient of the total winding area (i.e. slot area minus slot opening area) for the given layer.
    This function dispatches to `mutual_inductance_leakage_coefficient`, please see its documentation for a detailed explanation.

    # Panics
    Panics if the given layer index is larger than the total number of layers in the coil layout.
    */
    fn self_inductance_leakage_coefficient(&self, layer: u16, coil_layout: &CoilLayout) -> f64 {
        return self.mutual_inductance_leakage_coefficient(layer, layer, coil_layout);
    }

    /**
    Calculates the mutual inductance coefficient for `linked_layer` when a field is created by `excitation_layer` as described in section 3.5 of [MVP08] (p. 309 ff).

    The basic algorithm is as follows: The leakage inductance `L` of a coil inside the slot is the quotient of the flux linkage `Psi` and the current `i`.
    The flux linkage is the product of the leakage flux `Phi` and the number of turns of the coil `z_link`:
    `Psi = L * i = Phi * z_link`.

    The linkage coefficient is defined as
    `lambda = L / (mu_0 * z_link * z_exc)`
    where `mu_0` is the magnetic field constant, `z_link` is the number of turns of the linked layer (coil) and `z_exc` is the number of turns of the excitation coil.

    The linkage flux `Phi` can be calculated as `mu_0 * H * l`, `H` is the magnetic field strength and `l` is the axial length of the slot.

    For the calculation, it is assumed that the magnetic field strength in the iron core surrounding the slot is zero (e.g. the magnetic permeability is infinite).
    Furthermore, the flux in the slot is assumed to be horizontal from slot side to slot side, see fig. 3.5.2. in [MVP08]. Lastly, it is assumed that the number
    of conductors in a partial area of the coil equals the ratio of the partial area to the total coil area.

    If the linked layer (coil) and the excitation layer (coil) are on the same vertical height of the slot (case 1 in [MVP08], p. 316),
    both `H` and `z` are zero below the layers and are a function of the slot width `s` and the vertical coordinate (x) at the layer height in the general case.
    Above the layer, `H` continues to be a function of `s`, while `z` stays constant.
    This is especially true for the special case of self induction, where the linked layer and the excitation layer are identical.

    `H = z_exc(x) / z_exc * i_exc / s(x) = Delta_A_exc(x) / A_exc * z_exc * i_exc / s(x)`
    `z = z_link(x) = Delta_A_link(x) / A_link * z_exc`

    The partial area of the linked / excitation layer `Delta_A_exc/link(x)` is defined by the vertical coordinate and the slot geometry.
    For the special case of a rectangular slot, the relationships are particular simple:

    For `x` < lower limit of linked layer => `Delta_A_exc/link(x) = 0` and `z_link(x) = 0`
    => `Psi = 0`
    For `x` >= lower limit of linked layer and `x` <= upper limit of linked layer => `Delta_A_exc/link(x) ~ x` and `z_link(x) ~ x`
    => `Psi ~ x^2`
    For `x` > upper limit of linked layer => `Delta_A_exc/link(x) = A_exc/link` and `z_link(x) ~ z_link`
    => `Psi = const`

    If the linked layer (coil) is above the excitation layer (coil), case 2 in [MVP08], p. 316 applies:
    For `x` < lower limit of linked layer => `Delta_A_exc/link(x) ~ x` and `z_link(x) = 0`.
    => `Psi = 0`
    For `x` >= lower limit of linked layer and `x` <= upper limit of linked layer => `Delta_A_exc/link(x) = A_exc/link` and `z_link(x) ~ x`
    => `Psi ~ x`
    For `x` > upper limit of linked layer => `Delta_A_exc/link(x) = A_exc/link`
    => `Psi = const`

    If the linked layer (coil) is below the excitation layer (coil), case 2 in [MVP08], p. 316 applies:
    For `x` < lower limit of excitation layer => `Delta_A_exc/link(x) = 0` and `z_link(x) ~ x`.
    => `Psi = 0`
    For `x` >= lower limit of excitation layer and `x` <= upper limit of excitation layer => `Delta_A_exc/link(x) ~ x` and `z_link(x) = z_link`
    => `Psi ~ x`
    For `x` > upper limit of excitation layer => `Delta_A_exc/link(x) = A_exc/link`
    => `Psi = const`

    In the general case of an arbitrary slot shape, the following formulae can be used to calculate the leakage coefficient `lambda`:
    `lambda = Integral Delta_A_exc(x)/A_exc * Delta_A_link(x)/A_link * 1/s(x) dx`

    # Panics
    Panics if one of the given layer indices is larger than the total number of layers in the coil layout.
     */
    fn mutual_inductance_leakage_coefficient(
        &self,
        linked_layer: u16,
        excitation_layer: u16,
        coil_layout: &CoilLayout,
    ) -> f64 {
        // Check the relationship between the layers and adjust the calculation strategy
        let slot_contour_no_opening = self.contour_main_body();
        let slot_bounds_no_opening = slot_contour_no_opening.bounding_box();
        let slot_body_centroid = slot_contour_no_opening.centroid();

        let ordering = coil_layout.ordering_vertical(linked_layer, excitation_layer);
        let layer_bounds = match ordering {
            std::cmp::Ordering::Equal => {
                /*
                Both layers are located in the same height. This equals case 1 in [MVP08], p. 316.
                */
                layer_bounds_priv(
                    self,
                    linked_layer,
                    coil_layout,
                    slot_body_centroid,
                    &slot_bounds_no_opening,
                    1.0,
                    0.0,
                )
            }
            std::cmp::Ordering::Greater => {
                /*
                The linked layer is above the excitation layer. This equals case 2 in [MVP08], p. 316.
                */
                layer_bounds_priv(
                    self,
                    linked_layer,
                    coil_layout,
                    slot_body_centroid,
                    &slot_bounds_no_opening,
                    1.0,
                    0.0,
                )
            }
            std::cmp::Ordering::Less => {
                /*
                The linked layer is above the excitation layer. This equals case 2 in [MVP08], p. 316.
                */
                layer_bounds_priv(
                    self,
                    excitation_layer,
                    coil_layout,
                    slot_body_centroid,
                    &slot_bounds_no_opening,
                    1.0,
                    0.0,
                )
            }
        };

        let layer_contour = apply_bounds(&slot_contour_no_opening, &layer_bounds);
        let layer_area = layer_contour.area();

        return inductance_leakage_coefficient_priv(
            self,
            &slot_contour_no_opening,
            &slot_bounds_no_opening,
            &layer_contour,
            &layer_bounds,
            layer_area,
            &ordering,
        );
    }

    /**
    Calculates the leakage coefficient matrix of a slot for the given coil layout. This matrix is square and its
    numbers of rows / columns equals the number of layers that the given coil layout supports.

    The row contains the layer with the linked coil where the voltage due to the leakage flux is induced,
    while the column corresponds to the coil carrying the excitation coil ("excitation_layer").
    This means that the diagonal contains the self-inductance leakage coefficients,
    while the off-diagonals carry the mutual inductance leakage coefficients.

    The leakage coefficient matrix does not include the slot opening leakage nor the tooth tip leakage.
    These values can be calculated with the corresponding methods and then added to all elements of the matrix
    to get the complete leakage coefficient matrix.

    For a detailed explanation of the algorithm this function is based on, plese see the documentation of the `mutual_inductance_leakage_coefficient` method.
     */
    fn leakage_coefficient_matrix(&self, coil_layout: &CoilLayout) -> DMatrix<f64> {
        let layers = coil_layout.layers();
        let dimension = layers as usize;
        let mut matrix = DMatrix::repeat(dimension, dimension, 0.0);

        /*
        Precalculate some shared values
        */
        let slot_contour_no_opening = self.contour_main_body();
        let slot_bounds_no_opening = slot_contour_no_opening.bounding_box();
        let slot_body_centroid = slot_contour_no_opening.centroid();

        let all_layer_bounds: Vec<BoundingBox> = (0..layers)
            .into_par_iter()
            .map(|layer| {
                return layer_bounds_priv(
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

        let all_layer_contours: Vec<Contour> = all_layer_bounds
            .par_iter()
            .map(|bounds| apply_bounds(&slot_contour_no_opening, &bounds))
            .collect();

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

                *coefficient = inductance_leakage_coefficient_priv(
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

    /// Return the leakage coefficient proportion caused by the slot opening
    fn leakage_coefficient_opening(&self) -> f64 {
        if self.opening_width().get::<meter>() > 0.0 {
            return f64::from(self.magnetic_opening_height() / self.opening_width());
        } else {
            return 0.0; // TODO: Find better way to calculate this!
        }
    }

    /// Return the leakage coefficient proportion caused by the tooth tip
    /// leakage.
    fn leakage_coefficient_tooth_tip(&self, magnetic_air_gap: Length) -> f64 {
        if !self.consider_tooth_tip_leakage() || magnetic_air_gap.get::<meter>() <= 0.0 {
            return 0.0;
        } else {
            // Interpolation from 3.7.2 of [MVP08] (values read out by hand!)
            let x = vec![
                0.125, 0.275, 0.5, 0.55, 0.9, 1.0, 1.5, 1.65, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0, 10.0,
                12.0, 14.0, 16.0,
            ];
            let y = vec![
                1.0, 0.8, 0.65, 0.6, 0.4, 0.39, 0.25, 0.2, 0.14, 0.07, 0.0, -0.03, -0.08, -0.11,
                -0.13, -0.15, -0.16, -0.17,
            ];
            let len = y.len() - 1;
            let ml = vec![(y[len - 1] - y[len]) / (x[len - 1] - x[len])];
            let mr = vec![(y[0] - y[1]) / (x[0] - x[1])];
            let spline = AkimaSpline::new(x, y, Some(ml), Some(mr))
                .expect("spline can be constructed from given data");

            match spline.eval(f64::from(self.opening_width() / magnetic_air_gap)) {
                Some(val) => return val,
                None => 0.0,
            }
        }
    }

    /// Calculates the current displacement coefficients [kr, kx],
    /// where kr is the resistance increase coefficient and kx is the leakage
    /// inductance reduction coefficient.
    ///
    /// # OptimizationParameters
    /// - &self: Slot instance
    /// - frequency: Frequency of the electrical current
    /// - el_conductivity: Electrical conductivity
    /// - rel_permeability: Relative material permeability
    fn current_displacement_coefficients(
        &self,
        frequency: Frequency,
        el_conductivity: ElectricalConductivity,
        rel_permeability: f64,
    ) -> CurrentDisplacementCoefficients {
        let shapes = self.shapes(CoilLayout::Single, true);
        let [x_ul, y_ul, x_lr, y_lr] = self.slices(50, &shapes[0].contour());

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

    /// Returns the total slot area.
    fn area(&self) -> Area {
        return Area::new::<square_meter>(self.contour().area());
    }

    /// Returns the slot area available for winding (e.g. w/o slot opening
    /// area).
    fn winding_area(&self) -> Area {
        return Area::new::<square_meter>(self.contour_main_body().area());
    }

    /// Returns the slot width at the given slot height, starting at the air
    /// gap.
    fn width(
        &self,
        vertical_slot_coord: Length,
        contour: &Contour,
        slot_bounds: &BoundingBox,
    ) -> Length {
        // Case x = 0: width equals slot opening
        if vertical_slot_coord == Length::new::<meter>(0.0) {
            return self.opening_width();
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

    /// Returns the slot shapes.
    fn shapes(&self, coil_layout: CoilLayout, include_slot_opening: bool) -> Vec<Shape> {
        // Remove the slot opening, if necessary
        let contour = if include_slot_opening {
            self.contour()
        } else {
            self.contour_main_body()
        };

        // ==========================================================================
        // Return the path in case of a single-layer winding

        match coil_layout {
            CoilLayout::Single => {
                if let Ok(s) = Shape::from_outer(contour) {
                    return vec![s];
                }
                return Vec::new();
            }
            CoilLayout::DoubleHorizontal => {
                let bb = contour.bounding_box();
                let contour_u: Polysegment; // Contour of the upper layer
                let contour_l: Polysegment; // Contour of the lower layer

                let verts_par = [[0.0, bb.ymin() - 1.0], [0.0, bb.ymax() + 1.0]];
                let vertical_line = Polysegment::from_points(verts_par.as_slice());
                let mut separated_lines =
                    contour.intersection_cut(&vertical_line, DEFAULT_EPSILON, DEFAULT_MAX_ULPS);

                // Check which half has positive x-values
                let bb_first = separated_lines[0].bounding_box();
                if bb_first.xmin() >= 0.0 {
                    contour_l = separated_lines.pop().unwrap();
                    contour_u = separated_lines.pop().unwrap();
                } else {
                    contour_u = separated_lines.pop().unwrap();
                    contour_l = separated_lines.pop().unwrap();
                }

                let mut shapes = Vec::new();
                if let Ok(s) = Shape::from_outer(contour_l.into()) {
                    shapes.push(s);
                }
                if let Ok(s) = Shape::from_outer(contour_u.into()) {
                    shapes.push(s);
                }
                return shapes;
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
                let [contour_u, contour_l] = if bb_first.center()[1] >= center[1] {
                    let contour_l = separated_lines.pop().unwrap();
                    let contour_u = separated_lines.pop().unwrap();
                    [contour_u, contour_l]
                } else {
                    let contour_u = separated_lines.pop().unwrap();
                    let contour_l = separated_lines.pop().unwrap();
                    [contour_u, contour_l]
                };

                let mut shapes = Vec::new();
                if let Ok(s) = Shape::from_outer(contour_u.into()) {
                    shapes.push(s);
                }
                if let Ok(s) = Shape::from_outer(contour_l.into()) {
                    shapes.push(s);
                }
                return shapes;
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

                // Create shapes
                let mut shapes: Vec<Shape> = Vec::with_capacity(4);
                for contour in [contour_ll, contour_ul, contour_ur, contour_lr].into_iter() {
                    let mut contour = contour.expect("could not build slot shapes");

                    // Create full shape
                    let start = contour.segments().last().unwrap().stop();
                    match LineSegment::new(start, center, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                        Ok(ls) => contour.push_back(ls.into()),
                        Err(_) => (),
                    }

                    if let Ok(s) = Shape::from_outer(contour.into()) {
                        shapes.push(s);
                    }
                }
                return shapes;
            }
            CoilLayout::MultiVertical(layers) => {
                let mut shapes: Vec<Shape> = Vec::with_capacity(layers as usize);
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
                        // contour is then set as the new shap contour
                        if let Ok(shape) = Shape::from_outer(contour_u.into()) {
                            shapes.push(shape);
                        }
                        shape_contour = contour_l.into();
                    }
                }

                // Last shape
                if let Ok(shape) = Shape::from_outer(shape_contour) {
                    shapes.push(shape);
                }
                shapes
            }
        }
    }

    /**
    Return drawable shapes for the slot`.
     */
    #[cfg(feature = "cairo")]
    fn drawables(
        &self,
        coil_layout: CoilLayout,
        include_slot_opening: bool,
    ) -> Vec<DrawableCow<'_>> {
        let mut shape_style = Style::default();
        shape_style.background_color = crate::ORANGE;

        return self
            .shapes(coil_layout, include_slot_opening)
            .into_iter()
            .map(|shape| DrawableCow::new(shape, shape_style.clone()))
            .collect();
    }

    /// To calculate the current displacement factors, the slot is "sliced" into
    /// rectangles. This function can be used to visualize those slices.
    fn slice_shapes(&self, min_number_slices: usize) -> Vec<Shape> {
        let mut shapes = self.shapes(CoilLayout::Single, true);
        let [x_ul, y_ul, x_lr, y_lr] = self.slices(min_number_slices, &shapes[0].contour());

        for ii in 1..x_ul.len() {
            let pts = vec![
                [x_lr[ii], y_lr[ii]],
                [x_lr[ii], y_ul[ii]],
                [x_ul[ii], y_ul[ii]],
                [x_ul[ii], y_lr[ii]],
            ];
            let contour = Polysegment::from_points(&pts).into();
            if let Ok(s) = Shape::from_outer(contour) {
                shapes.push(s);
            }
        }
        return shapes;
    }

    /*
    Create a list of horizontal slices from a given slot. The slices are rectangles
    aligned with the x-y-coordinate system and defined by the x-y-coordinates of their
    upper left and lower right corner respectively. This function only covers slots
    w/o holes in them. Additionally, it is assumed that the slot is symmetric along
    the y-axis
    */
    fn slices(&self, min_number_slices: usize, contour: &Contour) -> [Vec<f64>; 4] {
        let mut poly = contour.polygonize(Polygonizer::PerType {
            arc: SegmentPolygonizer::MaximumAngle(TAU / 36.0),
            straight: SegmentPolygonizer::InnerSegments(1),
        });
        let max_slice_height = self.height().get::<meter>() / (min_number_slices as f64);

        // Middle of the vertical right side
        let mut x_cr: Vec<f64> = Vec::new();
        let mut y_cr: Vec<f64> = Vec::new();
        let mut h_slices: Vec<f64> = Vec::new();

        /*
        Loop over all polygon sections except the one connecting the last to the first
        vertex. All sections which are connected to a vertex with negative x-coordinate
        are ignored (assumption of a symmetric slot)
        */
        if let Some(mut pt1) = poly.next() {
            for pt2 in poly {
                // Stop loop once the "negative" half of the slot vertices (x < 0) is reached
                if pt1[0] < 0.0 || pt2[0] < 0.0 {
                    break;
                }

                // Skip sections which have a very small incline
                let delta_x = pt2[0] - pt1[0];
                let delta_y = pt2[1] - pt1[1];
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
                    x_cr.push(pt1[0] + (ii as f64 + 0.5) * delta_x / n_slices_section);
                    y_cr.push(pt1[1] + (ii as f64 + 0.5) * slice_height);
                    h_slices.push(slice_height);
                }

                // Prepare the next iteration
                pt1 = pt2;
            }
        }

        // Create the upper left and lower right corner vertices
        let number_slices = h_slices.len();
        let mut x_ul: Vec<f64> = Vec::with_capacity(number_slices);
        let mut y_ul: Vec<f64> = Vec::with_capacity(number_slices);
        let mut x_lr: Vec<f64> = Vec::with_capacity(number_slices);
        let mut y_lr: Vec<f64> = Vec::with_capacity(number_slices);

        for ii in 0..number_slices - 1 {
            x_ul.push(-x_cr[ii]);
            x_lr.push(x_cr[ii]);
            y_ul.push(y_cr[ii] + 0.5 * h_slices[ii]);
            y_lr.push(y_cr[ii] - 0.5 * h_slices[ii]);
        }

        // Reverse the order of the vectors so they start at the slot bottom
        x_ul.reverse();
        y_ul.reverse();
        x_lr.reverse();
        y_lr.reverse();

        return [x_ul, y_ul, x_lr, y_lr];
    }
}

dyn_clone::clone_trait_object!(Slot);

fn layer_bounds_priv<S: Slot + ?Sized>(
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
            // Tooth coil arangement => Two coils aranged horizontally next to each other
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

/// Internal function which is not meant to be called directly
fn inductance_leakage_coefficient_priv<S: Slot + ?Sized>(
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
        let width = slot.width(
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
        let width = slot.width(
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
        let width = slot.width(
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
Returns the area of the selected layer as a function of the vertical slot coordinate, which starts at the slot bottom.

# Panics
Panics if the given coil index is larger than the total number of coils in the coil layout.
*/
fn lower_part_of_layer_area(
    vertical_slot_coord: f64,
    layer_contour: &Contour,
    layer_bounds: &BoundingBox,
) -> f64 {
    if vertical_slot_coord >= layer_bounds.ymax() {
        return 0.0;
    }

    // Adjust the bounds if vertical_slot_coord is between ymin and ymax
    let layer_bounds = if vertical_slot_coord > layer_bounds.ymin() {
        BoundingBox::new(
            layer_bounds.xmin(),
            layer_bounds.xmax(),
            vertical_slot_coord,
            layer_bounds.ymax(),
        )
    } else {
        layer_bounds.clone()
    };

    return apply_bounds(&layer_contour, &layer_bounds).area();
}

pub fn angle_bottom_no_slope(angle_slot: f64) -> f64 {
    return FRAC_PI_2 - angle_slot / 2.0;
}
pub fn angle_top_no_slope(angle_slot: f64) -> f64 {
    return FRAC_PI_2 + angle_slot / 2.0;
}

pub fn angle_top_slope(angle_top: f64, angle_slot: f64) -> f64 {
    return angle_top - angle_slot / 2.0 - FRAC_PI_2;
}
pub fn angle_bottom_slope(angle_bottom: f64, angle_slot: f64) -> f64 {
    return angle_bottom + angle_slot / 2.0 - FRAC_PI_2;
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

pub fn angle_bottom_from_width_height(
    bottom_width: Length,
    side_bottom_width: Length,
    bottom_height: Length,
    angle_slot: f64,
) -> f64 {
    let struct_angle = AngleBottomFromWidthHeight {
        bottom_width,
        side_bottom_width,
        bottom_height,
        angle_slot,
    };
    return struct_angle.get();
}

/**
Helper struct to derive the bottom angle from the bottom width, side bottom width, bottom height and the slot angle
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
pub struct AngleBottomFromWidthHeight {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_bottom_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub bottom_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
}

impl AngleBottomFromWidthHeight {
    /**
    Get the bottom angle in rad
     */
    pub fn get(&self) -> f64 {
        let delta = 0.5 * (self.side_bottom_width - self.bottom_width);
        return self
            .bottom_height
            .get::<meter>()
            .atan2(delta.get::<meter>())
            + FRAC_PI_2
            - 0.5 * self.angle_slot;
    }
}

pub fn angle_top_from_width_height(
    top_width: Length,
    side_top_width: Length,
    top_height: Length,
    angle_slot: f64,
) -> f64 {
    let struct_angle = AngleTopFromWidthHeight {
        top_width,
        side_top_width,
        top_height,
        angle_slot,
    };
    return struct_angle.get();
}

/**
Helper struct to derive the top angle from the top width, side top width, top height and the slot angle
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
struct AngleTopFromWidthHeight {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub side_top_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub top_height: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_angle"))]
    pub angle_slot: f64,
}

impl AngleTopFromWidthHeight {
    /**
    Get the bottom angle in rad
     */
    pub fn get(&self) -> f64 {
        let delta = 0.5 * (self.side_top_width - self.top_width);
        return self.top_height.get::<meter>().atan2(delta.get::<meter>())
            + FRAC_PI_2
            + 0.5 * self.angle_slot;
    }
}

#[cfg(feature = "serde")]
pub(crate) mod serde_impl {
    use super::*;
    use deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError;
    struct Angle(f64);

    impl<'de> Deserialize<'de> for Angle {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = deserialize_angle(deserializer)?;
            return Ok(Self(value));
        }
    }

    pub(crate) fn deserialize_angle_bottom_from_width_height<'de, D>(
        deserializer: D,
    ) -> Result<f64, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(DeserializeUntaggedVerboseError)]
        enum AngleBottomFromWidthHeightDeserializer {
            AngleBottomFromWidthHeight(AngleBottomFromWidthHeight),
            Angle(Angle),
        }

        let _enum = AngleBottomFromWidthHeightDeserializer::deserialize(deserializer)?;
        match _enum {
            AngleBottomFromWidthHeightDeserializer::AngleBottomFromWidthHeight(
                deserializer_struct,
            ) => return Ok(deserializer_struct.get()),
            AngleBottomFromWidthHeightDeserializer::Angle(angle) => return Ok(angle.0),
        }
    }

    pub(crate) fn deserialize_angle_top_from_width_height<'de, D>(
        deserializer: D,
    ) -> Result<f64, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(DeserializeUntaggedVerboseError)]
        enum AngleTopFromWidthHeightDeserializer {
            AngleTopFromWidthHeight(AngleTopFromWidthHeight),
            Angle(Angle),
        }

        let _enum = AngleTopFromWidthHeightDeserializer::deserialize(deserializer)?;
        match _enum {
            AngleTopFromWidthHeightDeserializer::AngleTopFromWidthHeight(deserializer_struct) => {
                return Ok(deserializer_struct.get());
            }
            AngleTopFromWidthHeightDeserializer::Angle(angle) => return Ok(angle.0),
        }
    }
}

/**
This function calculates one side length of a semi-regular polygon. A semi-regular
polygon is a polygon which has 2*n sides, where n sides are of length a and n
sides are of length b. The sides a and b are alternating along the polygon surface.
The geometric dependencies are explained in detail in [Mat20b].
*/
pub fn semi_regular_polygon_side_length(
    first_side: f64,
    outer_radius: f64,
    number_of_sides: usize,
) -> Option<f64> {
    use num::Integer;
    if number_of_sides.is_odd() || first_side < 0.0 || outer_radius < 0.0 {
        return None;
    }

    let angle_first_side = 2.0 * (first_side / (2.0 * outer_radius)).asin();
    let angle_second_side = TAU / (number_of_sides as f64 / 2.0) - angle_first_side;
    return Some(2.0 * outer_radius * (angle_second_side / 2.0).sin());
}

/**
This function takes the given contour and "limits" it to the given bounding box.

The function is a relict from an earlier version of planar_geo and should
eventually be replaced by a more general solution within planar_geo.
 */
fn apply_bounds(contour: &Contour, bounding_box: &BoundingBox) -> Contour {
    #[derive(PartialEq, Clone, Copy, Debug)]
    enum BoundingBoxSide {
        Left,
        Right,
        Top,
        Bottom,
    }

    impl BoundingBoxSide {
        // Get the side of the bounding box where the given point is located.
        // If the point is directly on a bounding box corner, return the corresponding
        // vertical side. The point must be on one of the bounding box sides!
        fn new(point: [f64; 2], bounding_box: &BoundingBox) -> BoundingBoxSide {
            if point[0] == bounding_box.xmin() {
                if point[1] == bounding_box.ymax() {
                    return BoundingBoxSide::Top;
                } else {
                    return BoundingBoxSide::Left;
                }
            }
            if point[1] == bounding_box.ymin() {
                if point[0] == bounding_box.xmin() {
                    return BoundingBoxSide::Left;
                } else {
                    return BoundingBoxSide::Bottom;
                }
            }
            if point[0] == bounding_box.xmax() {
                if point[1] == bounding_box.ymin() {
                    return BoundingBoxSide::Bottom;
                } else {
                    return BoundingBoxSide::Right;
                }
            }
            if point[1] == bounding_box.ymax() {
                return BoundingBoxSide::Right;
            } else {
                return BoundingBoxSide::Top;
            }
        }

        fn adjacent_side(&self, clockwise: bool) -> Self {
            if clockwise {
                match self {
                    BoundingBoxSide::Bottom => return BoundingBoxSide::Left,
                    BoundingBoxSide::Left => return BoundingBoxSide::Top,
                    BoundingBoxSide::Top => return BoundingBoxSide::Right,
                    BoundingBoxSide::Right => return BoundingBoxSide::Bottom,
                }
            } else {
                match self {
                    BoundingBoxSide::Bottom => return BoundingBoxSide::Right,
                    BoundingBoxSide::Right => return BoundingBoxSide::Top,
                    BoundingBoxSide::Top => return BoundingBoxSide::Left,
                    BoundingBoxSide::Left => return BoundingBoxSide::Bottom,
                }
            }
        }

        fn corner(&self, bounding_box: &BoundingBox, clockwise: bool) -> [f64; 2] {
            if clockwise {
                match self {
                    BoundingBoxSide::Bottom => {
                        return [bounding_box.xmin(), bounding_box.ymin()];
                    }
                    BoundingBoxSide::Left => {
                        return [bounding_box.xmin(), bounding_box.ymax()];
                    }
                    BoundingBoxSide::Top => {
                        return [bounding_box.xmax(), bounding_box.ymax()];
                    }
                    BoundingBoxSide::Right => {
                        return [bounding_box.xmax(), bounding_box.ymin()];
                    }
                }
            } else {
                match self {
                    BoundingBoxSide::Bottom => {
                        return [bounding_box.xmax(), bounding_box.ymin()];
                    }
                    BoundingBoxSide::Right => {
                        return [bounding_box.xmax(), bounding_box.ymax()];
                    }
                    BoundingBoxSide::Top => {
                        return [bounding_box.xmin(), bounding_box.ymax()];
                    }
                    BoundingBoxSide::Left => {
                        return [bounding_box.xmin(), bounding_box.ymin()];
                    }
                }
            }
        }
    }

    fn try_add_segment(primitives: &mut Vec<Segment>, new_addition: Segment) {
        if new_addition.start() != new_addition.stop() {
            if let Some(primitive) = primitives.last() {
                if &new_addition != primitive {
                    primitives.push(new_addition);
                }
            } else {
                primitives.push(new_addition);
            }
        }
    }

    fn add_glue_segments(
        primitives: &mut Vec<Segment>,
        contour: &Contour,
        start: [f64; 2],
        stop: [f64; 2],
        bounding_box: &BoundingBox,
    ) {
        // Get the location of start and stop points on the bounding box
        let stop_side = BoundingBoxSide::new(stop, bounding_box);
        let start_side = BoundingBoxSide::new(start, bounding_box);

        // Short-circuit: If start and stop are on the same side, they might be
        // connected by a direct line. Calculate the middle point and check if
        // it is inside the segment_chain. If true, add the direct line and return
        let middle_point = [0.5 * (start[0] + stop[0]), 0.5 * (start[1] + stop[1])];

        // Check if the middle point is on the border of the bounding box AND inside the
        // polygon
        if ulps_eq!(
            middle_point[0],
            bounding_box.xmin(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) || ulps_eq!(
            middle_point[0],
            bounding_box.xmax(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) || ulps_eq!(
            middle_point[1],
            bounding_box.ymin(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) || ulps_eq!(
            middle_point[1],
            bounding_box.ymax(),
            epsilon = DEFAULT_EPSILON,
            max_ulps = DEFAULT_MAX_ULPS
        ) {
            if contour.covers_point(middle_point, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                if let Ok(ls) = LineSegment::new(stop, start, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
                    primitives.push(ls.into());
                }
                return ();
            }
        }

        // Add corners in a clockwise fashion until the start side is reached.
        // If one of the corners is not inside the original segment_chain, stop and
        // search in the counter-clockwise direction instead. It is sufficient
        // to check the first added corner!
        let mut clockwise = true;
        let first_corner = stop_side.corner(bounding_box, clockwise);

        let first_corner = if contour.covers_point(first_corner, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
        {
            first_corner
        } else {
            clockwise = false;
            stop_side.corner(bounding_box, clockwise)
        };

        // Add the first corner
        if let Ok(ls) = LineSegment::new(stop, first_corner, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
            primitives.push(ls.into());
        }

        // Loop until the start side has been reached
        let mut prev_side = stop_side;
        let mut prev_corner = first_corner;
        loop {
            // Check if start is located on the adjacent side
            let new_side = prev_side.adjacent_side(clockwise);
            if new_side == start_side {
                if let Ok(ls) =
                    LineSegment::new(prev_corner, start, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
                {
                    try_add_segment(primitives, ls.into());
                }
                return ();
            } else {
                let new_corner = new_side.corner(&bounding_box, clockwise);
                if let Ok(ls) =
                    LineSegment::new(prev_corner, new_corner, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
                {
                    try_add_segment(primitives, ls.into());
                }
                prev_side = new_side;
                prev_corner = new_corner;
            }
        }
    }

    // =========================================================================================================

    // Check if the bounding box contains any part of self
    let bb = contour.bounding_box();
    if bounding_box.approx_covers(&bb, DEFAULT_EPSILON, DEFAULT_MAX_ULPS) {
        // Fully contained
        return contour.clone();
    }
    if !bounding_box.intersects(&bb)
        && !bounding_box.approx_covers(&bb, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
        && !bb.approx_covers(&bounding_box, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
    {
        // Bounds do not contain any part of the segment_chain.
        return Polysegment::new().into();
    }

    /*
    The general idea is as follows: Perform an intersection cut with a segment_chain representing the bounding box.
    Then, filter away all polylines which are not inside the bounding box. After that, connect the start of each
    segment_chain with the end of its predecessor. Finally, close the segment_chain again if it was closed in the first place.
    */
    let cut_polylines = contour.polysegment().intersection_cut(
        Contour::from(bounding_box.clone()).polysegment(),
        DEFAULT_EPSILON,
        DEFAULT_MAX_ULPS,
    );

    let mut bound_primitives: Vec<Segment> = Vec::new();

    for pl in cut_polylines.into_iter() {
        for primitive in pl.into_iter() {
            let primitive_bounding_box = primitive.bounding_box();
            if bounding_box.approx_covers(
                &primitive_bounding_box,
                DEFAULT_EPSILON,
                DEFAULT_MAX_ULPS,
            ) {
                if let Some(prev) = bound_primitives.last() {
                    let stop = prev.stop();
                    let start = primitive.start();

                    if ulps_ne!(
                        start,
                        stop,
                        epsilon = DEFAULT_EPSILON,
                        max_ulps = DEFAULT_MAX_ULPS
                    ) {
                        add_glue_segments(
                            &mut bound_primitives,
                            contour,
                            start,
                            stop,
                            bounding_box,
                        );
                    }
                }

                try_add_segment(&mut bound_primitives, primitive);
            }
        }
    }

    // Add a corner, if necessary
    if bound_primitives.is_empty() {
        return Polysegment::new().into();
    }

    let start = bound_primitives.first().expect("is not empty").start();
    let stop = bound_primitives.last().expect("is not empty").stop();

    if ulps_ne!(
        start,
        stop,
        epsilon = DEFAULT_EPSILON,
        max_ulps = DEFAULT_MAX_ULPS
    ) {
        add_glue_segments(&mut bound_primitives, contour, start, stop, bounding_box);
    }

    return Polysegment::from_iter(bound_primitives.into_iter()).into();
}

// #[cfg(test)]
// mod tests {
//     use std::f64::consts::PI;

//     use crate::{RectangularSlot, SlotTrapezoidSemi};

//     use super::serde_impl::{
//         deserialize_angle_bottom_from_width_height,
// deserialize_angle_top_from_width_height,     };
//     use super::*;
//     use approx;
//     use indoc::indoc;
//     use uom::si::{area::square_meter, length::millimeter};

//     #[test]
//     fn test_semi_regular_polygon_side_length() {
//         // This is actually a regular polygon with 12 sides in total.
//         let first_side = 1.0;
//         let second_side = semi_regular_polygon_side_length(
//             first_side,
//             first_side * (2.0f64 + 3.0f64.sqrt()).sqrt(),
//             12,
//         )
//         .unwrap();
//         approx::assert_abs_diff_eq!(first_side, second_side);

//         // Now for an irregular polygon
//         let first_side = 1.0;
//         let second_side = semi_regular_polygon_side_length(first_side, 2.0,
// 12).unwrap();         approx::assert_abs_diff_eq!(1.070466, second_side,
// epsilon = 1e-6);

//         // And now some failed attempts
//         assert!(semi_regular_polygon_side_length(-1.0, 2.0, 12).is_none());
//         assert!(semi_regular_polygon_side_length(1.0, -2.0, 12).is_none());
//         assert!(semi_regular_polygon_side_length(1.0, 2.0, 11).is_none());
//     }

//     #[test]
//     fn test_area_calculation_trapezoid_slot() {
//         let slot = SlotTrapezoidSemi::new_without_slopes(
//             Length::new::<millimeter>(10.0),
//             Length::new::<millimeter>(2.0),
//             Length::new::<millimeter>(20.0),
//             Length::new::<millimeter>(2.0),
//             10.0 * PI / 180.0,
//             Length::new::<millimeter>(2.0),
//             Length::new::<millimeter>(1.0),
//             Length::new::<millimeter>(0.0),
//             true,
//         )
//         .unwrap();

//         let slot_contour_no_opening = slot.contour_main_body();

//         let slot_contour = slot.contour_main_body();

//         // Single layer
//         let layer_bounds = layer_bounds_priv(
//             &slot,
//             0,
//             &CoilLayout::Single,
//             slot_contour.centroid(),
//             &slot_contour.bounding_box(),
//             1.0,
//             0.0,
//         );

//         let layer_contour = slot_contour_no_opening
//             .apply_bounds(&layer_bounds, DEFAULT_EPSILON, DEFAULT_MAX_ULPS)
//             .unwrap();

//         let delta_area = lower_part_of_layer_area(10e-3, &layer_contour,
// &layer_bounds);         approx::assert_abs_diff_eq!(89.1529e-6, delta_area,
// epsilon = 1e-10);

//         let delta_area = lower_part_of_layer_area(5e-3, &layer_contour,
// &layer_bounds);         approx::assert_abs_diff_eq!(128.2168e-6, delta_area,
// epsilon = 1e-10);

//         let delta_area = lower_part_of_layer_area(3e-3, &layer_contour,
// &layer_bounds);         approx::assert_abs_diff_eq!(142.6175e-6, delta_area,
// epsilon = 1e-10);

//         let delta_area = lower_part_of_layer_area(2e-3, &layer_contour,
// &layer_bounds);         approx::assert_abs_diff_eq!(149.2063e-6, delta_area,
// epsilon = 0.001);     }

//     #[test]
//     fn test_area_calculation_rectangular_slot() {
//         let opening_height = Length::new::<millimeter>(1.0);
//         let opening_width = Length::new::<millimeter>(3.0);
//         let width = Length::new::<millimeter>(3.0);
//         let height = Length::new::<millimeter>(20.0);
//         let slot = RectangularSlot::new(width, opening_width, height,
// opening_height, true, false)             .unwrap();

//         let slot_contour = slot.contour_main_body();

//         // Single layer
//         let layer_bounds = layer_bounds_priv(
//             &slot,
//             0,
//             &CoilLayout::Single,
//             slot_contour.centroid(),
//             &slot_contour.bounding_box(),
//             1.0,
//             0.0,
//         );

//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(0.0, &slot_contour, &layer_bounds),
//             ((height - opening_height) * width).get::<square_meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(10e-3, &slot_contour, &layer_bounds),
//             10e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(5e-3, &slot_contour, &layer_bounds),
//             15e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(20e-3, &slot_contour, &layer_bounds),
//             0.0,
//             epsilon = 1e-6
//         );

//         // Double layer horizontal
//         let layer_bounds = layer_bounds_priv(
//             &slot,
//             0,
//             &CoilLayout::DoubleHorizontal,
//             slot_contour.centroid(),
//             &slot_contour.bounding_box(),
//             1.0,
//             0.0,
//         );

//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(10e-3, &slot_contour, &layer_bounds),
//             10e-3 * 0.5 * width.get::<meter>(),
//             epsilon = 1e-6
//         );

//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(12e-3, &slot_contour, &layer_bounds),
//             8e-3 * 0.5 * width.get::<meter>(),
//             epsilon = 1e-6
//         );

//         // Double layer vertical
//         let layer_bounds = layer_bounds_priv(
//             &slot,
//             0,
//             &CoilLayout::DoubleVertical,
//             slot_contour.centroid(),
//             &slot_contour.bounding_box(),
//             1.0,
//             0.0,
//         );

//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(10e-3, &slot_contour, &layer_bounds),
//             9.5e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(15e-3, &slot_contour, &layer_bounds),
//             5e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(5e-3, &slot_contour, &layer_bounds),
//             9.5e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );

//         let layer_bounds = layer_bounds_priv(
//             &slot,
//             1,
//             &CoilLayout::DoubleVertical,
//             slot_contour.centroid(),
//             &slot_contour.bounding_box(),
//             1.0,
//             0.0,
//         );

//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(9e-3, &slot_contour, &layer_bounds),
//             1.5e-3 * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(3e-3, &slot_contour, &layer_bounds),
//             (9.5e-3 - 2e-3) * width.get::<meter>(),
//             epsilon = 1e-6
//         );
//         approx::assert_abs_diff_eq!(
//             lower_part_of_layer_area(10.5e-3, &slot_contour, &layer_bounds),
//             0.0,
//             epsilon = 1e-6
//         );
//     }

//     #[derive(Deserialize)]
//     struct AngleBottomWrapper {
//         #[serde(deserialize_with =
// "deserialize_angle_bottom_from_width_height")]         angle: f64,
//     }

//     #[test]
//     fn test_deserialize_bottom_with_width_and_height() {
//         let data = indoc! {"
//         ---
//         angle:
//             bottom_width: 1.0 m
//             side_bottom_width: 3.0 m
//             bottom_height: 1.0 m
//             angle_slot: 10.0 deg
//         "};
//         let wrapper: AngleBottomWrapper =
// serde_yaml::from_str(data).unwrap();         let angle_slot = TAU / 36.0; //
// 10°         approx::assert_abs_diff_eq!(wrapper.angle, 0.75 * PI - 0.5 *
// angle_slot, epsilon = 1e-15);

//         let data = indoc! {"
//         ---
//         angle: 10.0 deg
//         "};
//         let wrapper: AngleBottomWrapper =
// serde_yaml::from_str(data).unwrap();         approx::assert_abs_diff_eq!
// (wrapper.angle, TAU / 36.0, epsilon = 1e-15);

//         let data = indoc! {"
//         ---
//         angle: 1.0
//         "};
//         let wrapper: AngleBottomWrapper =
// serde_yaml::from_str(data).unwrap();         approx::assert_abs_diff_eq!
// (wrapper.angle, 1.0, epsilon = 1e-15);     }

//     #[cfg_attr(feature = "serde", derive(Deserialize))]
//     struct AngleTopWrapper {
//         #[serde(deserialize_with =
// "deserialize_angle_top_from_width_height")]         angle: f64,
//     }

//     #[test]
//     fn test_deserialize_top_with_width_and_height() {
//         let data = indoc! {"
//         ---
//         angle:
//             top_width: 1.0
//             side_top_width: 3.0
//             top_height: 1.0
//             angle_slot: 10.0 deg
//         "};
//         let wrapper: AngleTopWrapper = serde_yaml::from_str(data).unwrap();
//         let angle_slot = TAU / 36.0; // 10°
//         approx::assert_abs_diff_eq!(wrapper.angle, 0.75 * PI + 0.5 *
// angle_slot, epsilon = 1e-15);

//         let data = indoc! {"
//         ---
//         angle: 10.0 deg
//         "};
//         let wrapper: AngleTopWrapper = serde_yaml::from_str(data).unwrap();
//         approx::assert_abs_diff_eq!(wrapper.angle, TAU / 36.0, epsilon =
// 1e-15);

//         let data = indoc! {"
//         ---
//         angle: 1.0
//         "};
//         let wrapper: AngleTopWrapper = serde_yaml::from_str(data).unwrap();
//         approx::assert_abs_diff_eq!(wrapper.angle, 1.0, epsilon = 1e-15);
//     }

//     #[test]
//     fn test_test_angle_bottom_from_width_height() {
//         let angle_slot = TAU / 36.0; // 10°

//         // Case: No slope (bottom_width = side_bottom_width)
//         approx::assert_abs_diff_eq!(
//             PI - 0.5 * angle_slot,
//             angle_bottom_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: Almost no slope
//         approx::assert_abs_diff_eq!(
//             PI - 0.5 * angle_slot,
//             angle_bottom_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(0.01),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 60°
//         approx::assert_abs_diff_eq!(
//             1.9471774,
//             angle_bottom_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(3.0),
//                 Length::new::<millimeter>(0.5),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 45°
//         approx::assert_abs_diff_eq!(
//             0.75 * PI - 0.5 * angle_slot,
//             angle_bottom_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(3.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 60°
//         approx::assert_abs_diff_eq!(
//             2.59067858,
//             angle_bottom_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(2.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );
//     }

//     #[test]
//     fn test_test_angle_top_from_width_height() {
//         let angle_slot = TAU / 36.0; // 10°

//         // Case: No slope (bottom_width = side_bottom_width)
//         approx::assert_abs_diff_eq!(
//             PI + 0.5 * angle_slot,
//             angle_top_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: Almost no slope
//         approx::assert_abs_diff_eq!(
//             PI + 0.5 * angle_slot,
//             angle_top_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(0.01),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 60°
//         approx::assert_abs_diff_eq!(
//             1.94717747 + angle_slot,
//             angle_top_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(3.0),
//                 Length::new::<millimeter>(0.5),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 45°
//         approx::assert_abs_diff_eq!(
//             0.75 * PI + 0.5 * angle_slot,
//             angle_top_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(3.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );

//         // Case: slope with 60°
//         approx::assert_abs_diff_eq!(
//             2.5906785 + angle_slot,
//             angle_top_from_width_height(
//                 Length::new::<millimeter>(1.0),
//                 Length::new::<millimeter>(2.0),
//                 Length::new::<millimeter>(1.0),
//                 angle_slot
//             ),
//             epsilon = 1e-6
//         );
//     }

//     #[test]
//     fn test_test_slot_side_bottom_and_top_width_from_rot_core() {
//         // Values from [Mat19] slot
//         let tooth_width = Length::new::<millimeter>(3.415);
//         let air_gap_radius = Length::new::<millimeter>(55.0);
//         let yoke_radius = Length::new::<millimeter>(85.0);
//         let slots = 36;
//         let side_height = Length::new::<millimeter>(17.0);
//         let opening_height = Length::new::<millimeter>(0.75);
//         let opening_width = Length::new::<millimeter>(2.0);

//         let [b_bottom, b_top] = slot_side_bottom_and_top_width_from_rot_core(
//             tooth_width,
//             air_gap_radius,
//             yoke_radius,
//             slots,
//             side_height,
//             opening_width,
//             opening_height,
//         );

//         approx::assert_abs_diff_eq!(b_bottom.get::<millimeter>(), 9.29996,
// epsilon = 1e-3);         approx::assert_abs_diff_eq!(b_top.
// get::<millimeter>(), 6.32535, epsilon = 1e-3);     }
// }
