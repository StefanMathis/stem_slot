pub mod coil_layout;
pub mod current_displacement;
pub mod error;
pub mod open_trapezoid;
pub mod rectangular;
pub mod semi_trapezoid;
pub mod slot;

pub const ORANGE: planar_geo::draw::Color = planar_geo::draw::Color {
    r: 1.0,
    g: 0.55,
    b: 0.0,
    a: 1.0,
};

pub mod prelude {
    /*!
    This module reexports all wire types defined in >TOFO, the
    [`Magnet`] trait as well as the [`stem_material::prelude`]
    module to simplify the usage of this crate.
     */

    pub use crate::coil_layout::CoilLayout;
    pub use crate::open_trapezoid::OpenTrapezoidSlot;
    pub use crate::rectangular::RectangularSlot;
    pub use crate::semi_trapezoid::SemiTrapezoidSlot;
    pub use crate::slot::Slot;

    pub use stem_material::prelude::*;
}
