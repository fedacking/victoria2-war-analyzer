use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::{Rgba, RgbaImage, imageops};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Overlay an SVG badge on top of every frame in an .ico file."
)]
struct Args {
    #[arg(long)]
    base_ico: PathBuf,

    #[arg(long)]
    overlay_svg: PathBuf,

    #[arg(long)]
    output_ico: PathBuf,

    #[arg(long, default_value_t = 0.36)]
    scale: f32,

    #[arg(long, value_enum, default_value_t = Anchor::BottomRight)]
    anchor: Anchor,

    #[arg(long, default_value_t = 0.06)]
    margin: f32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !(0.0 < args.scale && args.scale <= 1.0) {
        bail!("--scale must be greater than 0.0 and at most 1.0");
    }

    if !(0.0 <= args.margin && args.margin < 0.5) {
        bail!("--margin must be between 0.0 and 0.5");
    }

    let svg_bytes = std::fs::read(&args.overlay_svg).with_context(|| {
        format!(
            "failed to read overlay SVG '{}'",
            args.overlay_svg.display()
        )
    })?;
    let svg_tree = Tree::from_data(&svg_bytes, &Options::default())
        .with_context(|| format!("failed to parse SVG '{}'", args.overlay_svg.display()))?;

    let mut input = File::open(&args.base_ico)
        .with_context(|| format!("failed to open base icon '{}'", args.base_ico.display()))?;
    let icon_dir = IconDir::read(&mut input)
        .with_context(|| format!("failed to read ico file '{}'", args.base_ico.display()))?;

    let mut output_dir = IconDir::new(ResourceType::Icon);

    for entry in icon_dir.entries() {
        let icon_image = entry.decode().with_context(|| {
            format!(
                "failed to decode ico frame from '{}'",
                args.base_ico.display()
            )
        })?;
        let composed =
            overlay_on_frame(&icon_image, &svg_tree, args.scale, args.margin, args.anchor)?;
        output_dir.add_entry(IconDirEntry::encode(&composed)?);
    }

    let mut output = File::create(&args.output_ico).with_context(|| {
        format!(
            "failed to create output ico '{}'",
            args.output_ico.display()
        )
    })?;
    output_dir
        .write(&mut output)
        .with_context(|| format!("failed to write ico file '{}'", args.output_ico.display()))?;

    println!("Wrote {}", args.output_ico.display());

    Ok(())
}

fn overlay_on_frame(
    base_icon: &IconImage,
    svg_tree: &Tree,
    scale: f32,
    margin_ratio: f32,
    anchor: Anchor,
) -> Result<IconImage> {
    let width = base_icon.width();
    let height = base_icon.height();
    let mut base = RgbaImage::from_raw(width, height, base_icon.rgba_data().to_vec())
        .context("ico frame had unexpected pixel buffer size")?;

    let min_dimension = width.min(height);
    let overlay_size = ((min_dimension as f32) * scale).round().max(1.0) as u32;
    let margin = ((min_dimension as f32) * margin_ratio).round().max(0.0) as u32;

    let overlay = render_svg(svg_tree, overlay_size, overlay_size)?;
    let (x, y) = anchored_position(
        width,
        height,
        overlay.width(),
        overlay.height(),
        margin,
        anchor,
    );

    imageops::overlay(&mut base, &overlay, i64::from(x), i64::from(y));

    Ok(IconImage::from_rgba_data(width, height, base.into_raw()))
}

fn render_svg(svg_tree: &Tree, target_width: u32, target_height: u32) -> Result<RgbaImage> {
    let svg_size = svg_tree.size();
    let scale_x = target_width as f32 / svg_size.width();
    let scale_y = target_height as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    let render_width = (svg_size.width() * scale).round().max(1.0) as u32;
    let render_height = (svg_size.height() * scale).round().max(1.0) as u32;

    let mut pixmap = Pixmap::new(target_width, target_height)
        .with_context(|| format!("failed to allocate {target_width}x{target_height} pixmap"))?;

    let translate_x = ((target_width - render_width) as f32) / 2.0;
    let translate_y = ((target_height - render_height) as f32) / 2.0;
    let transform = Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(svg_tree, transform, &mut pixmap_mut);

    let mut image = RgbaImage::new(target_width, target_height);
    for (index, pixel) in pixmap.data().chunks_exact(4).enumerate() {
        let x = (index as u32) % target_width;
        let y = (index as u32) / target_width;
        image.put_pixel(x, y, Rgba([pixel[2], pixel[1], pixel[0], pixel[3]]));
    }

    Ok(image)
}

fn anchored_position(
    canvas_width: u32,
    canvas_height: u32,
    overlay_width: u32,
    overlay_height: u32,
    margin: u32,
    anchor: Anchor,
) -> (u32, u32) {
    let max_x = canvas_width.saturating_sub(overlay_width);
    let max_y = canvas_height.saturating_sub(overlay_height);

    let left = margin.min(max_x);
    let top = margin.min(max_y);
    let right = max_x.saturating_sub(margin);
    let bottom = max_y.saturating_sub(margin);
    let center_x = max_x / 2;
    let center_y = max_y / 2;

    match anchor {
        Anchor::TopLeft => (left, top),
        Anchor::TopRight => (right, top),
        Anchor::BottomLeft => (left, bottom),
        Anchor::BottomRight => (right, bottom),
        Anchor::Center => (center_x, center_y),
    }
}
