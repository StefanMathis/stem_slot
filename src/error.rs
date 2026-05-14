/*!
This module contains the [`Error`] enum, which represents the different ways
building one of the predefined slots can fail due to invalid input data. The
[`Error::Other`] variants supports arbitrary errors resulting from user-created
slot types.
*/

use compare_variables::Comparison;
use planar_geo::prelude::*;
use stem_material::uom::si::f64::Length;

/// An enum representing errors returned by [`Slot`](crate::slot::Slot)
/// constructors.
#[derive(Debug)]
pub enum Error {
    /**
    A given physical [`Length`] is not within its allowed value range (as
    specified inside the [`Comparison`], usually a length needs to be
    positive).
     */
    InvalidLength(Comparison<Length>),
    /// A given [`f64`] is not within its allowed value range.
    InvalidF64(Comparison<f64>),
    /// Failed to create a slot geometry due to the contained error.
    GeometryError(planar_geo::error::Error),
    /// Failed to create a slot geometry due to a self-intersection of its
    /// outline
    OutlineIntersection {
        intersection: Intersection,
        outline: Polysegment,
    },
    /// Fallback variant for arbitrary other errors (e.g. from custom
    /// [`Slot`](crate::slot::Slot) implementations).
    Other(Box<dyn std::error::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidLength(comparison) => comparison.fmt(f),
            Error::InvalidF64(comparison) => comparison.fmt(f),
            Error::GeometryError(error) => error.fmt(f),
            Error::OutlineIntersection {
                intersection,
                outline: _,
            } => {
                write!(
                    f,
                    "slot outline intersects itself (segments {} and {} intersect at {:?})",
                    intersection.left.segment_idx,
                    intersection.right.segment_idx,
                    intersection.point
                )
            }
            Error::Other(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<Comparison<Length>> for Error {
    fn from(value: Comparison<Length>) -> Self {
        return Error::InvalidLength(value);
    }
}

impl From<Comparison<f64>> for Error {
    fn from(value: Comparison<f64>) -> Self {
        return Error::InvalidF64(value);
    }
}

impl From<planar_geo::error::Error> for Error {
    fn from(value: planar_geo::error::Error) -> Self {
        return Error::GeometryError(value);
    }
}
