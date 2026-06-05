use cairo_viewport::{SideLength, Viewport};
use planar_geo::{
    draw::{Drawable, DrawableCow},
    prelude::*,
};
use std::{f64::consts::PI, path::PathBuf};
use stem_slot::{prelude::*, semi_trapezoid::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slices_comp()?;
    non_conform_slices()?;
    return Ok(());
}

fn non_conform_slices() -> Result<(), Box<dyn std::error::Error>> {
    // Manually draw two "illegal" slot geometries which require a custom impl
    // of Slot::slices.

    let non_symmetric: Contour = Polysegment::from_points(&[
        [1.0, 0.0],
        [1.0, -1.0],
        [2.0, -1.0],
        [2.0, -10.0],
        [-5.0, -10.0],
        [-2.0, -1.0],
        [-1.0, -1.0],
        [-1.0, 0.0],
    ])
    .into();

    let non_symmetric_text = Text::new(
        "Non-symmetric slot".into(),
        Anchor::Bottom,
        [0.0, 0.0],
        [0.0, 0.5],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    );

    let mut non_concave: Contour = Polysegment::from_points(&[
        [1.0, 0.0],
        [1.0, -1.0],
        [2.0, -1.0],
        [2.0, -10.0],
        [0.0, -8.0],
        [-2.0, -10.0],
        [-2.0, -1.0],
        [-1.0, -1.0],
        [-1.0, 0.0],
    ])
    .into();
    non_concave.translate([8.0, 0.0]);

    let non_concave_text = Text::new(
        "Non-concave slot".into(),
        Anchor::Bottom,
        [0.0, 0.0],
        [8.0, 0.5],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    );

    let bb = BoundingBox::new(-5.5, 11.5, -10.5, 1.5);
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(500));

    let fp =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!("docs/img/non_conform_slices.svg"));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        non_symmetric.draw(&stem_slot::SLOT_STYLE, cr)?;
        non_concave.draw(&stem_slot::SLOT_STYLE, cr)?;

        non_symmetric_text.draw(cr)?;
        non_concave_text.draw(cr)?;

        return Ok(());
    })?;

    return Ok(());
}

fn slices_comp() -> Result<(), Box<dyn std::error::Error>> {
    let bottom_radius = Length::new::<millimeter>(0.5);
    let slot_angle = PI / 18.0;
    let top_width = Length::new::<millimeter>(6.33381);
    let bottom_width = Length::new::<millimeter>(9.297);

    let slot: SemiTrapezoidSlot = SemiTrapezoidBuilder {
        bottom_width,
        top_width,
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        side_height: Length::new::<millimeter>(17.0),
        opening_height: Length::new::<millimeter>(0.75),
        slot_angle,
        bottom_angle: BottomAngle::new_no_slope(slot_angle),
        top_angle: TopAngle::new_no_slope(slot_angle),
        bottom_radius,
        bottom_side_radius: Length::new::<millimeter>(0.0),
        top_radius: Length::new::<millimeter>(1.0),
        top_side_radius: Length::new::<millimeter>(0.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()?;

    // Drawables of the regular slot
    let mut drawables = slot.drawables(&CoilLayout::Single, true);

    // Slices a
    let offset = 0.015;
    let bbs = slot.slices(10);
    drawables.extend(bbs.into_iter().map(|mut bb| {
        bb.translate([offset, 0.0]);
        DrawableCow::new(Contour::from(bb), stem_slot::SLOT_STYLE.clone())
    }));

    // Slices b
    let bbs = slot.slices(20);
    drawables.extend(bbs.into_iter().map(|mut bb| {
        bb.translate([2.0 * offset, 0.0]);
        DrawableCow::new(Contour::from(bb), stem_slot::SLOT_STYLE.clone())
    }));

    // Mirror the drawables
    let drawables: Vec<Drawable> = drawables
        .into_iter()
        .map(From::from)
        .map(|mut d: Drawable| {
            d.line_reflection([0.0, 0.0], [1.0, 0.0]);
            d
        })
        .collect();

    let bb = BoundingBox::new(-0.005, 0.035, -0.018, 0.002);
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(800));

    let mut texts: Vec<Text> = Vec::new();
    texts.push(Text::new(
        "Slot".into(),
        Anchor::Center,
        [0.0, 0.0],
        [0.0, bb.ymax() - 0.001],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "Minimum of 10 slices".into(),
        Anchor::Center,
        [0.0, 0.0],
        [offset, bb.ymax() - 0.001],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "Minimum of 20 slices".into(),
        Anchor::Center,
        [0.0, 0.0],
        [2.0 * offset, bb.ymax() - 0.001],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!("docs/img/slices_comp.svg"));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for drawable in drawables {
            drawable.draw(cr)?;
        }
        for text in texts {
            text.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}
