use compare_variables::Comparison;
use planar_geo::prelude::*;
use stem_material::uom::si::f64::Length;

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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidLength(comparison) => comparison.fmt(f),
            Error::InvalidF64(comparison) => comparison.fmt(f),
            Error::GeometryError(error) => error.fmt(f),
            Error::OutlineIntersection {
                intersection,
                outline,
            } => {
                write!(
                    f,
                    "slot outline intersects itself (segments {} and {} intersect at {:?})",
                    intersection.left.segment_idx,
                    intersection.right.segment_idx,
                    intersection.point
                )
            }
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
