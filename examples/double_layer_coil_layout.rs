use cairo_viewport::{SideLength, Viewport};
use planar_geo::{draw::Drawable, prelude::*};
use std::{f64::consts::PI, path::PathBuf};
use stem_slot::{prelude::*, semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    .unwrap();

    let offset = 0.015;
    let radius = 0.001;

    let hori: Vec<Drawable> = slot
        .drawables(&CoilLayout::DoubleHorizontal, false)
        .into_iter()
        .map(Drawable::from)
        .map(|mut d| {
            d.line_reflection([0.0, 0.0], [1.0, 0.0]);
            d
        })
        .collect();

    let vert: Vec<Drawable> = slot
        .drawables(&CoilLayout::DoubleVertical, false)
        .into_iter()
        .map(Drawable::from)
        .map(|mut d| {
            d.translate([offset, 0.0]);
            d.line_reflection([0.0, 0.0], [1.0, 0.0]);
            d
        })
        .collect();

    let mut bb = BoundingBox::from_bounded_entities(hori.iter().chain(vert.iter())).unwrap();
    bb.scale(1.01);
    bb.try_set_ymax(bb.ymax() + 0.0025);

    let view = Viewport::from_bounding_box(&bb, SideLength::Long(400));

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/double_layer_coil_layout.svg"));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for (idx, drawable) in hori.iter().enumerate() {
            drawable.draw(cr)?;

            let centroid = match &drawable.geometry {
                Geometry::Contour(contour) => contour.centroid(),
                _ => unreachable!(),
            };
            let circle: Contour = ArcSegment::circle(centroid, radius)
                .expect("positive radius")
                .into();
            let mut style = Style::default();
            style.background_color = Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };
            style.text = Some(Box::new(Text::new(
                idx.to_string(),
                Anchor::Center,
                [0.0, 0.0],
                [0.0, 0.0],
                Color::new(0.0, 0.0, 0.0, 1.0),
                16.0,
                0.0,
            )));
            circle.draw(&style, cr)?;
        }
        Text::new(
            "Distributed layout".into(),
            Anchor::Center,
            [0.0, 0.0],
            [0.0, bb.ymax() - 0.001],
            Color::new(0.0, 0.0, 0.0, 1.0),
            16.0,
            0.0,
        )
        .draw(cr)?;

        for (idx, drawable) in vert.iter().enumerate() {
            drawable.draw(cr)?;

            let centroid = match &drawable.geometry {
                Geometry::Contour(contour) => contour.centroid(),
                _ => unreachable!(),
            };
            let circle: Contour = ArcSegment::circle(centroid, radius)
                .expect("positive radius")
                .into();
            let mut style = Style::default();
            style.background_color = Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };
            style.text = Some(Box::new(Text::new(
                idx.to_string(),
                Anchor::Center,
                [0.0, 0.0],
                [0.0, 0.0],
                Color::new(0.0, 0.0, 0.0, 1.0),
                16.0,
                0.0,
            )));
            circle.draw(&style, cr)?;
        }
        Text::new(
            "Tooth-coil layout".into(),
            Anchor::Center,
            [0.0, 0.0],
            [offset, bb.ymax() - 0.001],
            Color::new(0.0, 0.0, 0.0, 1.0),
            16.0,
            0.0,
        )
        .draw(cr)?;

        return Ok(());
    })?;
    return Ok(());
}
