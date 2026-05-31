use cairo_viewport::{SideLength, Viewport};
use planar_geo::{draw::Drawable, prelude::*};
use std::{
    f64::consts::{PI, TAU},
    path::PathBuf,
};
use stem_slot::{
    open_trapezoid::OpenTrapezoidBuilder,
    prelude::*,
    semi_trapezoid::{SemiTrapezoidBuilder, SemiTrapezoidWithoutSlopesBuilder},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rectangular_slot = RectangularSlot::new(
        Length::new::<meter>(8.0),
        Length::new::<meter>(2.0),
        Length::new::<meter>(16.0),
        Length::new::<meter>(2.0),
        true,
    )?;

    let open_trapezoid_slot: OpenTrapezoidSlot = OpenTrapezoidBuilder {
        bottom_width: Length::new::<meter>(6.0),
        opening_width: Length::new::<meter>(6.0),
        height: Length::new::<meter>(16.0),
        side_height: Length::new::<meter>(12.0),
        opening_height: Length::new::<meter>(2.0),
        slot_angle: TAU / 36.0,
        bottom_radius: Length::new::<meter>(1.0),
        bottom_side_radius: Length::new::<meter>(3.0),
        consider_tooth_tip_leakage: true,
    }
    .try_into()?;

    let semi_trapezoid_slot_1: SemiTrapezoidSlot = SemiTrapezoidBuilder {
        bottom_width: Length::new::<meter>(5.0),
        top_width: Length::new::<meter>(6.0),
        opening_width: Length::new::<meter>(2.0),
        height: Length::new::<meter>(16.0),
        side_height: Length::new::<meter>(10.0),
        opening_height: Length::new::<meter>(2.0),
        slot_angle: TAU / 36.0,
        bottom_angle: (0.7 * PI).into(),
        top_angle: (0.8 * PI).into(),
        bottom_radius: Length::new::<meter>(0.5),
        bottom_side_radius: Length::new::<meter>(3.0),
        top_radius: Length::new::<meter>(0.5),
        top_side_radius: Length::new::<meter>(0.5),
        opening_radius: Length::new::<meter>(0.5),
        consider_tooth_tip_leakage: true,
    }
    .try_into()?;

    let semi_trapezoid_slot_2: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<meter>(10.0),
        opening_width: Length::new::<meter>(2.0),
        height: Length::new::<meter>(16.0),
        opening_height: Length::new::<meter>(2.0),
        slot_angle: TAU / 36.0,
        bottom_radius: Length::new::<meter>(0.5),
        top_radius: Length::new::<meter>(1.0),
        opening_radius: Length::new::<meter>(0.5),
        consider_tooth_tip_leakage: true,
    }
    .try_into()?;

    let offset = 15.0;

    let mut drawables: Vec<Drawable> = rectangular_slot
        .drawables(&CoilLayout::Single, true)
        .into_iter()
        .map(From::from)
        .collect();

    drawables.extend(
        open_trapezoid_slot
            .drawables(&CoilLayout::Single, true)
            .into_iter()
            .map(|d| {
                let mut d = Drawable::from(d);
                d.translate([offset, 0.0]);
                d
            }),
    );

    drawables.extend(
        semi_trapezoid_slot_1
            .drawables(&CoilLayout::Single, true)
            .into_iter()
            .map(|d| {
                let mut d = Drawable::from(d);
                d.translate([2.0 * offset, 0.0]);
                d
            }),
    );

    drawables.extend(
        semi_trapezoid_slot_2
            .drawables(&CoilLayout::Single, true)
            .into_iter()
            .map(|d| {
                let mut d = Drawable::from(d);
                d.translate([3.0 * offset, 0.0]);
                d
            }),
    );

    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb = BoundingBox::from_bounded_entities(drawables.iter().map(|d| d.bounding_box()))
        .ok_or("drawables is empty")?;
    bb.try_set_ymin(bb.ymin() - 0.1);
    bb.try_set_ymax(bb.ymax() + 3.0);
    bb.try_set_xmin(bb.xmin() - 0.2);
    bb.try_set_xmax(bb.xmax() + 0.8);

    let mut texts: Vec<Text> = Vec::new();
    texts.push(Text::new(
        "Rectangular slot".into(),
        Anchor::Center,
        [0.0, 0.0],
        [0.0, bb.ymax() - 2.0],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "Open trapezoid slot".into(),
        Anchor::Center,
        [0.0, 0.0],
        [offset, bb.ymax() - 2.0],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "Semi-closed trapezoid".into(),
        Anchor::Center,
        [0.0, 0.0],
        [2.0 * offset, bb.ymax() - 2.0],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "slot with slopes".into(),
        Anchor::Center,
        [0.0, 0.0],
        [2.0 * offset, bb.ymax() - 0.8],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "Semi-closed trapezoid".into(),
        Anchor::Center,
        [0.0, 0.0],
        [3.0 * offset, bb.ymax() - 2.0],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));
    texts.push(Text::new(
        "slot without slopes".into(),
        Anchor::Center,
        [0.0, 0.0],
        [3.0 * offset, bb.ymax() - 0.8],
        Color::new(0.0, 0.0, 0.0, 1.0),
        16.0,
        0.0,
    ));

    // let bb = BoundingBox::new(-0.005, 0.035, -0.018, 0.002);
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(800));

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/slot_types_showcase.svg"));

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
