use cairo_viewport::*;
use indoc::indoc;
use planar_geo::prelude::*;
use std::f64::consts::PI;
use stem_slot::{prelude::*, semi_trapezoid::*};

fn compare_to_reference<P: AsRef<std::path::Path>>(
    drawables: &[DrawableCow<'_>],
    path: P,
    view: Option<Viewport>,
) {
    let view = view.unwrap_or(
        Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap(),
    );
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, move |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;
            for drawable in drawables.iter() {
                drawable.draw(cr)?;
            }
            return Ok(());
        });
    };
    assert!(compare_or_create(path, callback, 0.99).is_ok());
}

#[test]
fn test_plot_without_slot_opening() {
    let angle_slot = PI / 3.0;
    let height = Length::new::<millimeter>(7.0);
    let opening_width = Length::new::<millimeter>(2.0);
    let opening_height = Length::new::<millimeter>(1.0);
    let bottom_width = Length::new::<millimeter>(15.3);

    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width,
        opening_width,
        height,
        opening_height,
        angle_slot,
        bottom_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    compare_to_reference(
        slot.drawables(CoilLayout::Single, false).as_slice(),
        "tests/img/semi_trapezoid_slot_wo_opening.png",
        None,
    );
}
