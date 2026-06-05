/*!
[`Slot`]: crate::slot::Slot
[`Slot::leakage_coefficient_opening`]: crate::slot::Slot::leakage_coefficient_opening
[`Slot::current_displacement_coefficients`]: crate::slot::Slot::current_displacement_coefficients
[`RectangularSlot`]: crate::rectangular::RectangularSlot
[`OpenTrapezoidSlot`]: crate::open_trapezoid::OpenTrapezoidSlot
[`SemiTrapezoidSlot`]: crate::semi_trapezoid::SemiTrapezoidSlot

Slot definition for stem - a Simulation Toolbox for Electric Motors.

 */
#![cfg_attr(feature = "doc-images",
cfg_attr(all(),
doc = ::embed_doc_image::embed_image!("current_displacement_coeffs_comp.svg", "docs/img/current_displacement_coeffs_comp.svg"),
doc = ::embed_doc_image::embed_image!("double_layer_coil_layout.svg", "docs/img/double_layer_coil_layout.svg"),
doc = ::embed_doc_image::embed_image!("slot_types_showcase.svg", "docs/img/slot_types_showcase.svg"),
doc = ::embed_doc_image::embed_image!("magnetic_core.png", "docs/img/magnetic_core.png"),
))]
#![cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
#![doc = include_str!("../docs/main.md")]
#![deny(missing_docs)]

pub mod coil_layout;
pub mod current_displacement;
pub mod error;
pub mod open_trapezoid;
pub mod rectangular;
pub mod semi_trapezoid;
pub mod slot;
pub use planar_geo;
pub use stem_material;

/**
Standard [`Color`](planar_geo::draw::Color) for drawing slots.

This color is used as the
[`Style::background_color`](planar_geo::draw::Style::background_color)s of the
[`DrawableCow`](planar_geo::draw::DrawableCow)s returned by
[`Slot::drawables`](crate::slot::Slot::drawables). The images of the different
slot types use this color.
 */
#[cfg(feature = "cairo")]
pub const ORANGE: planar_geo::draw::Color = planar_geo::draw::Color {
    r: 1.0,
    g: 0.55,
    b: 0.0,
    a: 1.0,
};

/// Default style for a slot contour.
#[cfg(feature = "cairo")]
pub const SLOT_STYLE: planar_geo::draw::Style = planar_geo::draw::Style {
    line_color: planar_geo::draw::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    },
    background_color: ORANGE,
    line_width: 0.5,
    line_style: planar_geo::draw::LineStyle::Solid,
    line_cap: planar_geo::draw::LineCap::Round,
    line_join: planar_geo::draw::LineJoin::Miter,
    text: None,
};

pub mod prelude {
    /*!
    This module reexports all slot types defined in this crate, the
    [`Slot`] trait, the [`BottomAngle`] and [`TopAngle`] enums as well as the
    [`stem_material::prelude`](https://docs.rs/stem_material/latest/stem_material/prelude/index.html)
    module.
     */

    pub use crate::coil_layout::CoilLayout;
    pub use crate::current_displacement::{
        CurrentDisplacementCalculator, CurrentDisplacementCoefficients,
    };
    pub use crate::open_trapezoid::OpenTrapezoidSlot;
    pub use crate::rectangular::RectangularSlot;
    pub use crate::semi_trapezoid::SemiTrapezoidSlot;
    pub use crate::slot::{BottomAngle, Slot, TopAngle};
    pub use planar_geo;
    pub use stem_material;

    // Prevent rustdoc from documenting the stem_material dependency
    #[doc(hidden)]
    pub use stem_material::prelude::*;
}
