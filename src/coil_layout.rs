/*!
This module provides the [`CoilLayout`] enum, which defines how the individual
coils / winding layers are positioned within a [`Slot`](crate::slot::Slot).
*/
use std::cmp::Ordering;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Index of the layer in the slot bottom, left side quadrant of a quadruple
/// layer winding. Is used in the definition of that winding and hence exposed
/// here. See [`CoilLayout`].
pub const QUADRUPLE_LAYER_BOTTOM_LEFT: u16 = 0;

/// Index of the layer in the slot top, left side quadrant of a quadruple layer
/// winding. Is used in the definition of that winding and hence exposed here.
/// See [`CoilLayout`].
pub const QUADRUPLE_LAYER_TOP_LEFT: u16 = 1;

/// Index of the layer in the slot top, right side quadrant of a quadruple layer
/// winding. Is used in the definition of that winding and hence exposed here.
/// See [`CoilLayout`].
pub const QUADRUPLE_LAYER_TOP_RIGHT: u16 = 2;

/// Index of the layer in the slot bottom, right side quadrant of a quadruple
/// layer winding. Is used in the definition of that winding and hence exposed
/// here. See [`CoilLayout`].
pub const QUADRUPLE_LAYER_BOTTOM_RIGHT: u16 = 3;

/**
An enum defining the position of individual coils / winding layers within a
[`Slot`](crate::slot::Slot).

This enum is used by various methods of [`Slot`](crate::slot::Slot) like e.g.
[`self_inductance_leakage_coefficient`](crate::slot::Slot::self_inductance_leakage_coefficient)
to represent the coil / layer positioning of different winding types. For
example, in a double-layer distributed winding, the two coils in a slot are
placed on top of each other (variant [`CoilLayout::DoubleVertical`]). By
contrast, a double-layer tooth-coil winding is represented by a
[`CoilLayout::DoubleHorizontal`]. The following drawing shows the layout for all
variants, using the example of a
[`RectangularSlot`](crate::rectangular::RectangularSlot).
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Coil layout variants][cad_coil_layout]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("cad_coil_layout", "docs/img/cad_coil_layout.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

In stem, the `Winding` trait from the
[stem_winding](https://crates.io/crates/stem_winding) crate requires the
implementation of a `coil_layout` method which returns the corresponding variant
of this enum. This is used to calculate properties like e.g. the slot leakage
inductance for motors.
 */
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoilLayout {
    /**
    A variant representing single-layer windings. The single coil / layer fills
    the entire winding area of the slot (but not the slot opening).
     */
    Single,
    /**
    A variant representing single-layer windings, usually casted squirrel-cage
    windings. The single coil / layer fills the slot completely (including the
    slot opening)
     */
    SingleFilled,
    /**
    A variant representing double layer windings (e.g. distributed windings).
    The coil in the first layer is placed at the slot bottom, the one in the
    second layer at the slot top.
     */
    DoubleVertical,
    /**
    A variant representing double layer windings (e.g. tooth-coil windings).
    The coil in the first layer is placed on the left side of the slot, the one
    in the second layer on the right side.
     */
    DoubleHorizontal,
    /**
    A variant representing a quadruple-layer winding, which is essentially a
    tooth-coil winding where the coils at the two slot sides are split in the
    middle again. The individual coils / layers are placed in a clockwise order,
    starting on the left side of the slot bottom.
     */
    Quadruple,
    /**
    A variant representing a winding with an arbitrary number of layers, which
    is equal to the value of the anonymous field of the variant. The individual
    layers are placed on top of each other, starting with the first layer at the
    slot bottom (see the drawing in the enum docstring). The height of the
    individual slot slices is identical.
     */
    MultiVertical(u16),
}

impl CoilLayout {
    /**
    Returns the number of layers for `self`.

    # Examples

    ```
    use stem_slot::coil_layout::CoilLayout;

    assert_eq!(CoilLayout::Single.layers(), 1);
    assert_eq!(CoilLayout::SingleFilled.layers(), 1);
    assert_eq!(CoilLayout::DoubleVertical.layers(), 2);
    assert_eq!(CoilLayout::DoubleHorizontal.layers(), 2);
    assert_eq!(CoilLayout::Quadruple.layers(), 4);
    assert_eq!(CoilLayout::MultiVertical(5).layers(), 5);
    ```
     */
    pub const fn layers(&self) -> u16 {
        return match self {
            CoilLayout::Single => 1,
            CoilLayout::SingleFilled => 1,
            CoilLayout::DoubleVertical => 2,
            CoilLayout::DoubleHorizontal => 2,
            CoilLayout::Quadruple => 4,
            CoilLayout::MultiVertical(val) => *val,
        };
    }

    /**
    Returns the vertical position of the `first_layer` relative to the
    `second_layer` (as seen from the slot bottom)

    # Panics
    Panics if one of the layers is equal to or larger than the total number of
    layers of `self` (see [`CoilLayout::layers`]).

    # Examples

    ## DoubleVertical

    A [`CoilLayout::DoubleVertical`] aranges the first layer at the slot bottom
    and the second at the slot top. Hence, the first layer is "lesser" compared
    to the second.
    ```
    use stem_slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::DoubleVertical;

    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(1, 0));
    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 0));
    ```

    ## DoubleHorizontal

    A [`CoilLayout::DoubleHorizontal`] aranges both layers in the same vertical
    position. Hence, both layers are "equal".
    ```
    use stem_slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::DoubleHorizontal;

    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(1, 0));
    ```

    ## Quadruple

    A [`CoilLayout::Quadruple`] arranges its layers as follows:
    ```ignore
      0  3  |  1  2  |     <- slot bottom
      1  2  |  0  3  |     <- slot top
            |        |     <- air gap
     slot 0 | slot 1 | ...
    ```

    Correspondingly, 0 is equal to 3, 1 is equal to 2, and both 0 and 3 are
    lesser than 1 and 2:

    ```
    use stem_slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::Quadruple;

    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 3));
    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(2, 3));
    ```

    ## MultiVertical

    A [`CoilLayout::MultiVertical`] arranges the all layers on top of each
    other, starting at the slot bottom.
    ```
    use stem_slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::MultiVertical(3);

    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 2));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(2, 1));
    ```
     */
    pub fn ordering_vertical(&self, first_layer: u16, second_layer: u16) -> Ordering {
        assert!(first_layer < self.layers());
        assert!(second_layer < self.layers());
        match self {
            CoilLayout::Single => return Ordering::Equal,
            CoilLayout::SingleFilled => return Ordering::Equal,
            CoilLayout::DoubleVertical => return first_layer.cmp(&second_layer),
            CoilLayout::DoubleHorizontal => return Ordering::Equal,
            CoilLayout::Quadruple => {
                if first_layer == second_layer {
                    return Ordering::Equal;
                }
                if first_layer == QUADRUPLE_LAYER_BOTTOM_LEFT
                    || first_layer == QUADRUPLE_LAYER_BOTTOM_RIGHT
                {
                    if second_layer == QUADRUPLE_LAYER_BOTTOM_LEFT
                        || second_layer == QUADRUPLE_LAYER_BOTTOM_RIGHT
                    {
                        return Ordering::Equal;
                    } else {
                        return Ordering::Less;
                    }
                } else {
                    if second_layer == QUADRUPLE_LAYER_BOTTOM_LEFT
                        || second_layer == QUADRUPLE_LAYER_BOTTOM_RIGHT
                    {
                        return Ordering::Greater;
                    } else {
                        return Ordering::Equal;
                    }
                }
            }
            CoilLayout::MultiVertical(_) => return first_layer.cmp(&second_layer),
        }
    }

    /**
    Returns true if this [`CoilLayout`] variant uses the slot opening as space
    for conductors and false otherwise.

    # Examples

    ```
    use stem_slot::coil_layout::CoilLayout;

    // True for these coil layouts:
    assert!(CoilLayout::SingleFilled.includes_slot_opening());

    // False for all of these:
    assert!(!CoilLayout::Single.includes_slot_opening());
    assert!(!CoilLayout::DoubleHorizontal.includes_slot_opening());
    assert!(!CoilLayout::DoubleVertical.includes_slot_opening());
    assert!(!CoilLayout::Quadruple.includes_slot_opening());
    assert!(!CoilLayout::MultiVertical(5).includes_slot_opening());
    ```
     */
    pub fn includes_slot_opening(&self) -> bool {
        match self {
            CoilLayout::SingleFilled => true,
            _ => false,
        }
    }
}
