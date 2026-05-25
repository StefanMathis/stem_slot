stem_slot
=========

<!-- This file has ben generated with build.rs by concatenating docs/links.md,
docs/main.md and (if available docs/end.md). Do not modify this file, instead
modify the components. -->

[`Slot`]: https://docs.rs/stem_slot/0.1.0/stem_slot/slot/trait.Slot.html
[current_displacement_coeffs_comp.svg]: https://raw.githubusercontent.com/StefanMathis/stem_slot/refs/heads/main/docs/img/current_displacement_coeffs_comp.svg

[![Documentation](https://docs.rs/stem_slot/badge.svg)](https://docs.rs/stem_slot)

Winding slot definition definition for stem - a Simulation Toolbox for Electric Motors.

The full API documentation is available at <https://docs.rs/stem_slot/0.1.0/stem_slot>.

> **Feedback welcome!**  
> Found a bug, missing docs, or have a feature request?  
> Please open an issue on [GitHub](https://github.com/StefanMathis/stem_slot.git).

TODO

# `Slot` trait

## Geometry

## Slot leakage factors

## Current displacement

![Current displacement coefficient comparison][current_displacement_coeffs_comp.svg]

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