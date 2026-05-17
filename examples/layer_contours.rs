use cairo_viewport::{SideLength, Viewport};
use planar_geo::{draw::Drawable, prelude::*};
use std::{f64::consts::PI, path::PathBuf};
use stem_slot::{prelude::*, semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<millimeter>(10.0),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(20.0),
        opening_height: Length::new::<millimeter>(2.0),
        angle_slot: 10.0 * PI / 180.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        top_radius: Length::new::<millimeter>(1.0),
        opening_radius: Length::new::<millimeter>(0.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()
    .unwrap();

    let mut entire_slot = slot.drawables(&CoilLayout::Single, true)[0].clone();
    entire_slot.style.background_color.a = 0.4;
    let mut drawables = vec![
        entire_slot.clone(),
        entire_slot.clone(),
        entire_slot.clone(),
        entire_slot.clone(),
    ];
    drawables.extend(slot.drawables(&CoilLayout::Quadruple, true));

    // Mirror the drawables
    let offset = 0.015;
    let drawables: Vec<Drawable> = drawables
        .into_iter()
        .map(From::from)
        .enumerate()
        .map(|(i, mut d): (usize, Drawable)| {
            let mult = i % 4;
            d.line_reflection([0.0, 0.0], [1.0, 0.0]);
            d.translate([offset * mult as i32 as f64, 0.0]);
            d
        })
        .collect();

    let mut bb = BoundingBox::from_bounded_entities(drawables.iter()).unwrap();
    bb.try_set_ymax(bb.ymax() + 0.0025);

    let mut texts: Vec<Text> = Vec::new();
    for i in 0..4 {
        texts.push(Text::new(
            format!("Layer {i}"),
            Anchor::Center,
            [0.0, 0.0],
            [i as i32 as f64 * offset, bb.ymax() - 0.001],
            Color::new(0.0, 0.0, 0.0, 1.0),
            16.0,
            0.0,
        ));
    }
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(800));

    let fp =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!("docs/img/layer_contours.svg"));
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
