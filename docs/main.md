> **Feedback welcome!**  
> Found a bug, missing docs, or have a feature request?  
> Please open an issue on [GitHub](https://github.com/StefanMathis/stem_slot.git).

Magnetic cores of electric motor often feature "grooves" along the air gap where
the coils of electric windings are located. These grooves are typically called
"slots". The image below shows the rotor core of an asynchronous machine with
long and thin slots meant for a cast squirrel-cage winding:

![Magnetic core][magnetic_core.png]

This crate provides the [`Slot`] trait, which requires the definition of the
geometric slot extents from the implementor. From that information, it can
derive various physical properties which influence the operating behaviour of
an electric motor:
- Space available for the coils and by extent the ohmic resistance of the
winding
- Leakage inductance coefficients for the tooth tip, slot opening, self- and
mutual inductance etc.
- Current displacement factors for massive conductors filling the entire slot
(usually the case for cast squirrel-cage windings).

The [Physical property calculation](#physical-property-calculation) section
discusses these features in-depth.

This crate is part of the stem (Simulation Toolbox for Electric Motors)
framework, where those properites are used in higher-level crates to calculate
the operational behaviour of a motor. See the
[stem book](https://stefanmathis.github.io/stem_book/) for an introduction.

# Predefined slot types

Besides the [`Slot`] trait, this crate also provides a bunch of predefined slot
types:

![Slot type showcase][slot_types_showcase.svg]

- Rectangular slot: [`RectangularSlot`]
- Open trapezoid slot: [`OpenTrapezoidSlot`]
- Semi-closed trapezoid slot: [`SemiTrapezoidSlot`]

These slots are highly customizable, see their respective docstring for details.

# Physical property calculation

When an alternating current flows through the conductors in a slot, it creates
an alternating magnetic field which influences the operating behaviour of the
motor. The [`Slot`] trait offers fast analytical calculation routines taken from
standard literature for electrical machines such as e.g. [\[1\]](#1) for some of
these effects. These methods usually assume that the core material surrounding
the slot is magnetically "superconducting", i.e. made out of ferromagnetic
material whose magnetic resistance / reluctance can be neglected compared to
that of the air / conductors.

## Slot leakage factors

Parts of the magnetic field created by the slot conductors closes over the slot
instead of passing over the air gap. These magnetic fluxes create no magnetic
force or torque, but still induce voltages into the conductors. Mathematically,
this influence can be expressed as "leakage" inductances which are the product
of a slot-geometry dependent dimensionless coefficient and a slot-independent
inductance derived from the winding. These coefficients can be analytically
calculated with [`Slot`] trait methods such as
[`Slot::leakage_coefficient_opening`]. The following example shows how to get
the self- and mutual inductance coefficients for individual layers in a
multi-layered winding:

![Double-layered coil-layout][double_layer_coil_layout.svg]

```rust
use std::f64::consts::PI;
use approx::assert_abs_diff_eq;
use stem_slot::{prelude::*, semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder};

// Trapezoid slot shown in the image above
let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
    bottom_width: Length::new::<millimeter>(10.0),
    opening_width: Length::new::<millimeter>(2.0),
    height: Length::new::<millimeter>(15.0),
    opening_height: Length::new::<millimeter>(2.0),
    slot_angle: 12.0 * PI / 180.0,
    bottom_radius: Length::new::<millimeter>(2.0),
    top_radius: Length::new::<millimeter>(1.0),
    opening_radius: Length::new::<millimeter>(0.0),
    consider_tooth_tip_leakage: true,
}
.try_into()
.expect("given parameters create a valid slot");

// Coefficients for a distributed two-layer coil layout
let coeffs_distr = slot.leakage_coefficient_matrix(&CoilLayout::DoubleHorizontal);
assert_abs_diff_eq!(coeffs_distr[(0, 0)], 0.5903, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_distr[(1, 0)], 0.5903, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_distr[(0, 1)], 0.5903, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_distr[(1, 1)], 0.5903, epsilon=1e-3);

// Coefficients for a tooth-coil two-layer coil layout
let coeffs_toco = slot.leakage_coefficient_matrix(&CoilLayout::DoubleVertical);
assert_abs_diff_eq!(coeffs_toco[(0, 0)], 1.0869, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_toco[(1, 0)], 0.4589, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_toco[(0, 1)], 0.4589, epsilon=1e-3);
assert_abs_diff_eq!(coeffs_toco[(1, 1)], 0.3188, epsilon=1e-3);
```

## Current displacement coefficients

If a conductor fills the entire slot (usually the case for squirrel-cage
windings), the current distribution along the conductor cross section becomes
uneven due to the self-inductance, resulting in an effective higher resistance
and lower inductance. The [`Slot::current_displacement_coefficients`] method
uses a semi-numerical approach to calculate coefficients for resistance and
inductance which separates the slot into multiple stacked "slices". As the graph
below shows, the higher the number of slices, the more precise the result
becomes at the cost of a longer calculation time.

![Current displacement coefficient comparison][current_displacement_coeffs_comp.svg]

This particular plot shows the coefficients for an open rectangular slot. For
this kind of slot, an exact analytical solution exists which can be used to
benchmark the semi-numerical approach.

# Serialization and deserialization

If the `serde` feature is enabled, all slot types from this crate can be
serialized and deserialized. During deserialization, the invariants are
validated (to e.g. prevent negative slot height).

Units and quantities can be deserialized from strings representing SI units via
the [dyn_quantity](https://crates.io/crates/dyn_quantity) crate. Similarily,
it is possible to serialize the quantities of a wire as value-unit strings using
the [serialize_with_units](https://docs.rs/dyn_quantity/latest/dyn_quantity/quantity/serde_impl/fn.serialize_with_units.html) function.

See the chapter [serialization and deserialization](https://stefanmathis.github.io/stem_book/serialization_and_deserialization.html) of the [stem book](https://stefanmathis.github.io/stem_book/)
for details.

# Acknowledgments

The technical drawings used in the docstrings have been created using 
LibreCAD (<https://librecad.org/>).

# Literature

<a id="1">\[1\]</a>
Müller, Germar; Vogt, Karl; Ponick, Bernd: Berechnung elektrischer
Maschinen, 6th edition (2008), Wiley-VCH, Weinheim