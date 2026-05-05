use std::cmp::Ordering;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/**
The "coil layout" describes how the coils of a winding are arranged in a slot.
 */
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoilLayout {
    /**
    Only applicable for single-layer windings. The single coil of a slot fills it entirely.
     */
    Single,
    /**
    Only applicable for double-layer windings. The coil in the first layer is placed at the slot bottom, the one in the second layer at the slot top.
     */
    DoubleVertical,
    /**
    Only applicable for double-layer windings. The coil in the first layer is placed on the left side of the slot, the one in the second layer on the right side.
     */
    DoubleHorizontal,
    /**
    Only applicable to quadruple-layer windings. The coils are placed as follows:
    `0  1` <- slot bottom
    `2  3` <- slot top
     */
    Quadruple,
    /**
    The slot is vertically separated into multiple sections of identical height.
     */
    MultiVertical(u16),
}

impl CoilLayout {
    /**
    Returns the number of layers associated with the coil layout.
     */
    pub fn layers(&self) -> u16 {
        return match self {
            CoilLayout::Single => 1,
            CoilLayout::DoubleVertical => 2,
            CoilLayout::DoubleHorizontal => 2,
            CoilLayout::Quadruple => 4,
            CoilLayout::MultiVertical(val) => *val,
        };
    }

    /**
    Returns the vertical position of a layer compared to another layer. The coordinate system starts at the slot bottom.

    # Panics
    Panics if one of the layers is equal to or larger than the total number of layers of the `self`. This property can be requested by the `layers` method.

    # Examples

    A `CoilLayout::DoubleVertical` aranges the first layer at the bottom and the second at the top. Hence, the first layer is "lesser" compared to the second.
    ```
    use slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::DoubleVertical;

    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(1, 0));
    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 0));
    ```

    A `CoilLayout::DoubleHorizontal` aranges both layers in the same vertical position. Hence, both layers are "equal".
    ```
    use slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::DoubleHorizontal;

    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(1, 0));
    ```

    A `CoilLayout::Quadruple` aranges its layers as follows:
      1  2  |  1  2  |
      0  3  |  0  3  |
            |        |
     slot 0 | slot 1 | ...
    ```
    use slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::Quadruple;

    assert_eq!(std::cmp::Ordering::Equal, coil_layout.ordering_vertical(0, 3));
    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(2, 3));
    ```

    A `CoilLayout::MultiVertical` aranges the all layers on top of each other, starting at the slot bottom
    ```
    use slot::coil_layout::CoilLayout;

    let coil_layout = CoilLayout::MultiVertical(3);

    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 1));
    assert_eq!(std::cmp::Ordering::Less, coil_layout.ordering_vertical(0, 2));
    assert_eq!(std::cmp::Ordering::Greater, coil_layout.ordering_vertical(2, 1));
    ```
     */
    pub fn ordering_vertical(&self, first_layer: u16, second_layer: u16) -> Ordering {
        if first_layer == second_layer {
            return Ordering::Equal; // Holds true for all coil layouts
        }
        assert!(first_layer < self.layers());
        assert!(second_layer < self.layers());
        match self {
            CoilLayout::Single => unreachable!(), // Caught by first_layer == second_layer
            CoilLayout::DoubleVertical => return first_layer.cmp(&second_layer),
            CoilLayout::DoubleHorizontal => return Ordering::Equal,
            CoilLayout::Quadruple => {
                if first_layer == QUADRUPLE_LAYER_LOWER_LEFT
                    || first_layer == QUADRUPLE_LAYER_LOWER_RIGHT
                {
                    if second_layer == QUADRUPLE_LAYER_LOWER_LEFT
                        || second_layer == QUADRUPLE_LAYER_LOWER_RIGHT
                    {
                        return Ordering::Equal;
                    } else {
                        return Ordering::Less;
                    }
                } else {
                    if second_layer == QUADRUPLE_LAYER_LOWER_LEFT
                        || second_layer == QUADRUPLE_LAYER_LOWER_RIGHT
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
}

pub const QUADRUPLE_LAYER_LOWER_LEFT: u16 = 0;
pub const QUADRUPLE_LAYER_UPPER_LEFT: u16 = 1;
pub const QUADRUPLE_LAYER_UPPER_RIGHT: u16 = 2;
pub const QUADRUPLE_LAYER_LOWER_RIGHT: u16 = 3;
pub const DOUBLE_HORIZONTAL_LAYER_LEFT: u16 = 0;
pub const DOUBLE_HORIZONTAL_LAYER_RIGHT: u16 = 1;
