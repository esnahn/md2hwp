// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Writes a usvg tree as an EMF+ Dual metafile: every drawing step is recorded
//! once as EMF+ (exact, with alpha) and once as plain GDI for players without
//! EMF+. Every shape is a filled path: strokes are outlined first, so pen
//! semantics never apply. The GDI half cannot hold alpha and draws such fills
//! opaque. Anything else that needs compositing is reported as unsupported.
//!
//! The EMF+ record layout follows what Windows GDI+ records for the same calls.

use resvg::usvg::{
    self,
    tiny_skia_path::{Path, PathBuilder, PathSegment, PathStroker, Point, Rect, Transform},
};

/// GDI logical units per CSS pixel; the window/viewport extents map them back.
const SCALE: i32 = 64;
const LIMIT: f32 = (1 << 30) as f32 / SCALE as f32;
/// Longest side, in pixels, of the bitmap copy written for GDI players.
const GDI_IMAGE_SIDE: u32 = 1024;
/// Most tiles one pattern fill may draw.
const MAX_TILES: u64 = 4096;
const DESCRIPTION: &str = "md2hwp\0SVG\0\0";
const HEADER_SIZE: usize = 108 + 2 * DESCRIPTION.len();
/// The single GDI object handle used for the current brush.
const BRUSH: i32 = 1;
const STOCK_WHITE_BRUSH: i32 = 0x8000_0000_u32 as i32;
const STOCK_NULL_PEN: i32 = 0x8000_0008_u32 as i32;
const PLUS_VERSION: u32 = 0xdbc0_1002;

pub fn is_generated(bytes: &[u8]) -> bool {
    let mark: Vec<u8> = "md2hwp\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    bytes.len() >= HEADER_SIZE
        && bytes[..4] == 1u32.to_le_bytes()
        && bytes[40..44] == 0x464d_4520u32.to_le_bytes()
        && bytes[64..68] == 108u32.to_le_bytes()
        && bytes[108..108 + mark.len()] == mark
}

/// Returns the EMF bytes, or the first feature that has no form in this writer.
pub fn encode(tree: &usvg::Tree) -> Result<Vec<u8>, String> {
    let (width, height) = (tree.size().width(), tree.size().height());
    let (device_width, device_height) = (width.ceil() as i32, height.ceil() as i32);
    let mut writer = Writer::default();
    // Header flag 1: dual. Data: version, display reference device, 96 dpi.
    writer.plus(0x4001, 1, &words(&[PLUS_VERSION, 1, 96, 96]));
    writer.plus(0x401e, 9, &[]); // SetAntiAliasMode: AntiAlias8x4
    // No SetPixelOffsetMode: with Half, GDI+ plays the picture half a pixel up and left.
    // The specification names no default page unit: state pixels, scale 1.
    writer.plus(0x4030, 2, &1.0f32.to_le_bytes()); // SetPageTransform: UnitTypePixel
    writer.record(17, &[8]); // SETMAPMODE MM_ANISOTROPIC
    writer.record(10, &[0, 0]); // SETWINDOWORGEX
    writer.record(9, &[device_width * SCALE, device_height * SCALE]); // SETWINDOWEXTEX
    writer.record(12, &[0, 0]); // SETVIEWPORTORGEX
    writer.record(11, &[device_width, device_height]); // SETVIEWPORTEXTEX
    writer.record(18, &[1]); // SETBKMODE TRANSPARENT
    writer.record(37, &[STOCK_NULL_PEN]);
    // An SVG viewport clips its content.
    let mut viewport = Vec::new();
    for value in [0.0, 0.0, width, height] {
        viewport.extend_from_slice(&value.to_le_bytes());
    }
    writer.plus(0x4032, 0, &viewport); // SetClipRect, replace
    let logical = |length: f32| (length * SCALE as f32).round() as i32;
    writer.record(30, &[0, 0, logical(width), logical(height)]); // INTERSECTCLIPRECT
    writer.children(tree.root(), Transform::identity())?;
    writer.release_brush();
    writer.plus(0x4002, 0, &[]); // EndOfFile
    writer.record(14, &[0, 0, 20]); // EOF

    let frame = |length: f32| (f64::from(length) * 2540.0 / 96.0).round() as i32;
    let mut header = Vec::with_capacity(HEADER_SIZE);
    let mut put = |values: &[i32]| {
        for value in values {
            header.extend_from_slice(&value.to_le_bytes());
        }
    };
    put(&[1, HEADER_SIZE as i32]);
    put(&[0, 0, device_width - 1, device_height - 1]); // rclBounds, inclusive pixels
    put(&[0, 0, frame(width), frame(height)]); // rclFrame, 0.01 mm
    put(&[0x464d_4520, 0x0001_0000]);
    put(&[
        (HEADER_SIZE + writer.bytes.len()) as i32,
        writer.count + 1,
        2, // handles: the reserved entry and the brush
        DESCRIPTION.len() as i32,
        108,
        0,
    ]);
    // A 96 dpi reference device whose pixel and millimetre sizes are both exact.
    put(&[1920, 960, 508, 254]);
    put(&[0, 0, 0]);
    put(&[508_000, 254_000]);
    for unit in DESCRIPTION.encode_utf16() {
        header.extend_from_slice(&unit.to_le_bytes());
    }
    header.extend_from_slice(&writer.bytes);
    Ok(header)
}

fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// An opaque GDI colour and the ARGB value EMF+ fills with.
#[derive(Clone, Copy)]
struct Color {
    gdi: i32,
    argb: u32,
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    count: i32,
    /// EMF+ records waiting to be wrapped in one GDI comment record.
    pending: Vec<u8>,
    brush: Option<i32>,
    fill_mode: Option<i32>,
    saves: u32,
    /// Opacity of the enclosing group, which holds a single shape.
    fade: Option<f32>,
}

impl Writer {
    /// Appends one EMF+ record. GDI+ counts the padding in DataSize as well.
    fn plus(&mut self, kind: u16, flags: u16, data: &[u8]) {
        let padded = data.len().next_multiple_of(4) as u32;
        self.pending.extend_from_slice(&kind.to_le_bytes());
        self.pending.extend_from_slice(&flags.to_le_bytes());
        self.pending.extend_from_slice(&(12 + padded).to_le_bytes());
        self.pending.extend_from_slice(&padded.to_le_bytes());
        self.pending.extend_from_slice(data);
        self.pending
            .resize(self.pending.len() + padded as usize - data.len(), 0);
    }

    fn record(&mut self, kind: i32, values: &[i32]) {
        if !self.pending.is_empty() {
            // GDICOMMENT carrying the EMF+ records recorded since the last one.
            let pending = std::mem::take(&mut self.pending);
            self.bytes.extend_from_slice(&70i32.to_le_bytes());
            self.bytes
                .extend_from_slice(&(16 + pending.len() as u32).to_le_bytes());
            self.bytes
                .extend_from_slice(&(4 + pending.len() as u32).to_le_bytes());
            self.bytes.extend_from_slice(b"EMF+");
            self.bytes.extend_from_slice(&pending);
            self.count += 1;
        }
        self.bytes.extend_from_slice(&kind.to_le_bytes());
        self.bytes
            .extend_from_slice(&(8 + 4 * values.len() as i32).to_le_bytes());
        for value in values {
            self.bytes.extend_from_slice(&value.to_le_bytes());
        }
        self.count += 1;
    }

    /// A GDI record made of 32-bit fields followed by bytes, padded to four.
    fn record_with_bytes(&mut self, kind: i32, values: &[i32], bytes: &[u8]) {
        let padding = bytes.len().next_multiple_of(4) - bytes.len();
        self.record(kind, values);
        let start = self.bytes.len() - 4 * values.len() - 4;
        let size = (8 + 4 * values.len() + bytes.len() + padding) as u32;
        self.bytes[start..start + 4].copy_from_slice(&size.to_le_bytes());
        self.bytes.extend_from_slice(bytes);
        self.bytes.resize(self.bytes.len() + padding, 0);
    }

    fn release_brush(&mut self) {
        if self.brush.take().is_some() {
            self.record(37, &[STOCK_WHITE_BRUSH]); // SELECTOBJECT
            self.record(40, &[BRUSH]); // DELETEOBJECT
        }
    }

    fn children(&mut self, group: &usvg::Group, transform: Transform) -> Result<(), String> {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => self.group(group, transform)?,
                usvg::Node::Path(path) => self.path(path, transform)?,
                usvg::Node::Text(text) => self.group(text.flattened(), transform)?,
                usvg::Node::Image(image) => self.image(image, transform)?,
            }
        }
        Ok(())
    }

    /// Draws a raster image. EMF+ keeps the encoded file and its alpha; GDI gets
    /// an uncompressed copy of limited size whose transparency is a 1-bit mask.
    fn image(&mut self, image: &usvg::Image, transform: Transform) -> Result<(), String> {
        if !image.is_visible() {
            return Ok(());
        }
        if self.fade.is_some() {
            return Err("an image under group opacity".into());
        }
        let encoded: &[u8] = match image.kind() {
            usvg::ImageKind::PNG(data) | usvg::ImageKind::JPEG(data) => data,
            usvg::ImageKind::GIF(_) => return Err("an embedded GIF image".into()),
            usvg::ImageKind::WEBP(_) => return Err("an embedded WebP image".into()),
            usvg::ImageKind::SVG(_) => return Err("a nested SVG image".into()),
        };
        let pixels = image::load_from_memory(encoded)
            .map_err(|_| "an embedded image that cannot be decoded".to_owned())?
            .into_rgba8();
        let (width, height) = (image.size().width(), image.size().height());
        let corner = |x: f32, y: f32| {
            let mut point = Point::from_xy(x, y);
            transform.map_point(&mut point);
            point
        };
        let corners = [
            corner(0.0, 0.0),
            corner(width, 0.0),
            corner(0.0, height),
            corner(width, height),
        ];
        if corners
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > LIMIT || p.y.abs() > LIMIT)
        {
            return Err("coordinates outside the EMF range".into());
        }

        // EmfPlusImage: bitmap whose data is the compressed file itself.
        let mut object = words(&[PLUS_VERSION, 1, 0, 0, 0, 0, 1]);
        object.extend_from_slice(encoded);
        self.plus(0x4008, 0x0501, &object); // Object: image, id 1
        // InterpolationMode: nearest neighbour, bilinear or high quality bicubic,
        // following what resvg does for each image-rendering value.
        let interpolation = match image.rendering_mode() {
            usvg::ImageRendering::OptimizeSpeed
            | usvg::ImageRendering::CrispEdges
            | usvg::ImageRendering::Pixelated => 5,
            usvg::ImageRendering::Smooth => 3,
            usvg::ImageRendering::OptimizeQuality | usvg::ImageRendering::HighQuality => 7,
        };
        self.plus(0x4021, interpolation, &[]); // SetInterpolationMode
        // Smooth interpolation reads beyond the image; mirror it there instead of
        // fading its edges into transparency (ImageAttributes, WrapMode TileFlipXY).
        let attributes = if interpolation == 5 {
            0xffff_ffff
        } else {
            self.plus(0x4008, 0x0802, &words(&[PLUS_VERSION, 0, 3, 0, 0, 0]));
            2
        };
        // DrawImagePoints: no attributes, source in pixels, destination parallelogram.
        let mut draw = words(&[attributes, 2]);
        // GDI+ puts source pixel centres on whole coordinates, so the image's
        // edge is half a pixel before the first centre.
        for value in [-0.5, -0.5, pixels.width() as f32, pixels.height() as f32] {
            draw.extend_from_slice(&value.to_le_bytes());
        }
        draw.extend_from_slice(&3u32.to_le_bytes());
        for point in &corners[..3] {
            draw.extend_from_slice(&point.x.to_le_bytes());
            draw.extend_from_slice(&point.y.to_le_bytes());
        }
        self.plus(0x401b, 1, &draw);

        // GDI: a world transform places the bitmap, whatever the rotation.
        let small = if pixels.width().max(pixels.height()) > GDI_IMAGE_SIDE {
            let scale = GDI_IMAGE_SIDE as f32 / pixels.width().max(pixels.height()) as f32;
            let side = |length: u32| ((length as f32 * scale).round() as u32).max(1);
            image::imageops::resize(
                &pixels,
                side(pixels.width()),
                side(pixels.height()),
                image::imageops::FilterType::Triangle,
            )
        } else {
            pixels
        };
        let (columns, rows) = (small.width() as i32, small.height() as i32);
        let place = transform
            .pre_scale(width / columns as f32, height / rows as f32)
            .post_scale(SCALE as f32, SCALE as f32);
        let float = |value: f32| value.to_bits() as i32;
        let bounds = |pick: fn(&Point) -> f32, low: bool| {
            let values = corners.iter().map(pick);
            let edge = if low {
                values.fold(f32::MAX, f32::min)
            } else {
                values.fold(f32::MIN, f32::max)
            };
            edge.floor() as i32
        };
        let bounds = [
            bounds(|p| p.x, true),
            bounds(|p| p.y, true),
            bounds(|p| p.x, false),
            bounds(|p| p.y, false),
        ];
        self.release_brush();
        self.record(33, &[]); // SAVEDC
        self.record(21, &[3]); // SETSTRETCHBLTMODE COLORONCOLOR
        self.record(
            35, // SETWORLDTRANSFORM
            &[
                float(place.sx),
                float(place.ky),
                float(place.kx),
                float(place.sy),
                float(place.tx),
                float(place.ty),
            ],
        );
        let translucent = small.pixels().any(|pixel| pixel.0[3] < 128);
        // Rows are stored bottom-up and padded to four bytes.
        let row_bytes = |bits: usize| (columns as usize * bits).div_ceil(32) * 4;
        let blit = |writer: &mut Self, bits: u16, palette: &[u8], data: &[u8], operation: u32| {
            let mut info = Vec::with_capacity(40 + palette.len());
            info.extend_from_slice(&40u32.to_le_bytes());
            info.extend_from_slice(&columns.to_le_bytes());
            info.extend_from_slice(&rows.to_le_bytes());
            info.extend_from_slice(&1u16.to_le_bytes());
            info.extend_from_slice(&bits.to_le_bytes());
            info.extend_from_slice(&0u32.to_le_bytes());
            info.extend_from_slice(&(data.len() as u32).to_le_bytes());
            info.extend_from_slice(&[0; 8]);
            info.extend_from_slice(&(palette.len() as u32 / 4).to_le_bytes());
            info.extend_from_slice(&0u32.to_le_bytes());
            info.extend_from_slice(palette);
            let header = 80;
            let values = [
                bounds[0],
                bounds[1],
                bounds[2],
                bounds[3],
                0,
                0,
                0,
                0,
                columns,
                rows,
                header,
                info.len() as i32,
                header + info.len() as i32,
                data.len() as i32,
                0, // DIB_RGB_COLORS
                operation as i32,
                columns,
                rows,
            ];
            info.extend_from_slice(data);
            writer.record_with_bytes(81, &values, &info); // STRETCHDIBITS
        };
        if translucent {
            // SRCAND with a mask that is black where the image is opaque.
            let stride = row_bytes(1);
            let mut mask = vec![0u8; stride * rows as usize];
            for (x, y, pixel) in small.enumerate_pixels() {
                if pixel.0[3] < 128 {
                    let row = (rows as u32 - 1 - y) as usize;
                    mask[row * stride + x as usize / 8] |= 0x80 >> (x % 8);
                }
            }
            blit(self, 1, &[0, 0, 0, 0, 255, 255, 255, 0], &mask, 0x0088_00c6);
        }
        let stride = row_bytes(24);
        let mut colors = vec![0u8; stride * rows as usize];
        for (x, y, pixel) in small.enumerate_pixels() {
            let [red, green, blue, alpha] = pixel.0;
            if alpha >= 128 {
                let at = (rows as u32 - 1 - y) as usize * stride + x as usize * 3;
                colors[at..at + 3].copy_from_slice(&[blue, green, red]);
            }
        }
        // SRCPAINT over the masked area, or SRCCOPY for an opaque image.
        let operation = if translucent {
            0x00ee_0086
        } else {
            0x00cc_0020
        };
        blit(self, 24, &[], &colors, operation);
        self.record(34, &[-1]); // RESTOREDC
        self.fill_mode = None;
        Ok(())
    }

    fn group(&mut self, group: &usvg::Group, transform: Transform) -> Result<(), String> {
        let transform = transform.pre_concat(group.transform());
        if group.blend_mode() != usvg::BlendMode::Normal {
            return Err("a blend mode".into());
        }
        if group.mask().is_some() {
            return Err("a mask".into());
        }
        if !group.filters().is_empty() {
            return Err("a filter".into());
        }
        let outer = self.fade;
        let opacity = group.opacity().get();
        if opacity != 1.0 {
            // One shape can be faded through its colours; several shapes would
            // need to be composited first.
            if outer.is_some() {
                return Err("nested group opacity".into());
            }
            if !holds_one_path(group) {
                // Shapes of one opaque colour, such as the glyphs of a text,
                // composite to their union in that colour.
                let mut union = PathBuilder::new();
                let mut paint = None;
                if !same_fill(group, transform, &mut union, &mut paint)? {
                    return Err("group opacity over several shapes".into());
                }
                let (Some(paint), Some(union)) = (paint, union.finish()) else {
                    return Ok(());
                };
                let color = solid(paint, opacity, "fill")?;
                return self.group_content(group, transform, |writer| {
                    writer.fill(
                        &union,
                        usvg::FillRule::NonZero,
                        color,
                        Transform::identity(),
                    )
                });
            }
            self.fade = Some(opacity);
        }
        let drawn =
            self.group_content(group, transform, |writer| writer.children(group, transform));
        self.fade = outer;
        drawn
    }

    /// Draws `content` under the group's clip path, if it has one.
    fn group_content(
        &mut self,
        group: &usvg::Group,
        transform: Transform,
        content: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let Some(clip) = group.clip_path() else {
            return content(self);
        };
        if clip.clip_path().is_some() {
            return Err("a nested clip path".into());
        }
        let mut shapes = Vec::new();
        clip_shapes(
            clip.root(),
            transform.pre_concat(clip.transform()),
            &mut shapes,
        )?;
        let [(shape, rule, shape_transform)] = shapes.as_slice() else {
            return Err("a clip path made of several shapes".into());
        };
        let Some(shape) = placed(shape, *shape_transform)? else {
            return Ok(()); // An empty clip shows nothing.
        };
        self.clipped(&shape, *rule, false, content)
    }

    /// Draws `content` inside `shape`, or outside it when `exclude` is set.
    fn clipped(
        &mut self,
        shape: &Path,
        rule: usvg::FillRule,
        exclude: bool,
        content: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        // CombineMode Intersect/Exclude and the matching RGN_AND/RGN_DIFF.
        let (plus_mode, gdi_mode) = if exclude { (0x0400, 4) } else { (0x0100, 1) };
        // RestoreDC brings back the selection made before SaveDC, so no brush of
        // this writer may be selected across either boundary.
        self.release_brush();
        let save = self.saves;
        self.saves += 1;
        self.plus(0x4025, 0, &words(&[save])); // Save
        self.plus_path(shape, rule);
        self.plus(0x4033, plus_mode, &[]); // SetClipPath, object 0
        self.record(33, &[]); // SAVEDC
        self.fill_mode(rule);
        self.gdi_path(shape);
        self.record(67, &[gdi_mode]); // SELECTCLIPPATH
        content(self)?;
        self.release_brush();
        self.plus(0x4026, 0, &words(&[save])); // Restore
        self.record(34, &[-1]); // RESTOREDC
        self.fill_mode = None;
        Ok(())
    }

    fn path(&mut self, path: &usvg::Path, transform: Transform) -> Result<(), String> {
        if !path.is_visible() {
            return Ok(());
        }
        let fade = self.fade.unwrap_or(1.0);
        let fill = match path.fill() {
            Some(fill) => Some((
                fill.rule(),
                ink(fill.paint(), fill.opacity().get() * fade, "fill")?,
            )),
            None => None,
        };
        let stroke = match path.stroke() {
            Some(stroke) => Some((
                stroke_outline(path.data(), stroke, transform),
                ink(stroke.paint(), stroke.opacity().get() * fade, "stroke")?,
                stroke.opacity().get(),
            )),
            None => None,
        };
        let draw_fill = |writer: &mut Self| match fill {
            Some((rule, ink)) => writer.paint(path.data(), rule, ink, transform),
            None => Ok(()),
        };
        let draw_stroke = |writer: &mut Self| match &stroke {
            Some((Some(outline), ink, _)) => {
                writer.paint(outline, usvg::FillRule::NonZero, *ink, transform)
            }
            _ => Ok(()),
        };
        let stroke_last = matches!(path.paint_order(), usvg::PaintOrder::FillAndStroke);
        if fade != 1.0
            && let (Some(_), Some((outline, _, opacity))) = (&fill, &stroke)
        {
            // A faded shape is composited as one: where the stroke covers the
            // fill only the stroke shows, so the fill is drawn outside it.
            if !stroke_last || *opacity != 1.0 {
                return Err("group opacity over a translucent or reordered stroke".into());
            }
            let covered = match outline {
                Some(outline) => placed(outline, transform)?,
                None => None,
            };
            if let Some(covered) = covered {
                self.clipped(&covered, usvg::FillRule::NonZero, true, draw_fill)?;
                return draw_stroke(self);
            }
        }
        if stroke_last {
            draw_fill(self).and_then(|()| draw_stroke(self))
        } else {
            draw_stroke(self).and_then(|()| draw_fill(self))
        }
    }

    fn paint(
        &mut self,
        shape: &Path,
        rule: usvg::FillRule,
        ink: Ink,
        transform: Transform,
    ) -> Result<(), String> {
        match ink {
            Ink::Solid(color) => self.fill(shape, rule, color, transform),
            Ink::Pattern(pattern) => self.tile(shape, rule, pattern, transform),
        }
    }

    /// Fills `shape` with a pattern: the shape becomes a clip and the pattern's
    /// content is drawn once for every tile that touches the shape.
    fn tile(
        &mut self,
        shape: &Path,
        rule: usvg::FillRule,
        pattern: &usvg::Pattern,
        transform: Transform,
    ) -> Result<(), String> {
        if self.fade.is_some() {
            return Err("a pattern under group opacity".into());
        }
        let cell = pattern.rect();
        let (width, height) = (cell.width(), cell.height());
        let Some(clip) = placed(shape, transform)? else {
            return Ok(());
        };
        // The shape's extent in pattern space selects the tiles.
        let area = pattern
            .transform()
            .invert()
            .and_then(|inverse| shape.bounds().transform(inverse));
        let Some(area) = area else {
            return Ok(());
        };
        let columns = tile_range(area.left(), area.right(), cell.x(), width);
        let rows = tile_range(area.top(), area.bottom(), cell.y(), height);
        if columns.clone().count() as u64 * rows.clone().count() as u64 > MAX_TILES {
            return Err("a pattern repeated too many times".into());
        }
        // Content that stays inside its tile needs no clip of its own.
        let content = pattern.root().stroke_bounding_box();
        let slack = 1e-3 * width.max(height);
        let contained = content.left() >= -slack
            && content.top() >= -slack
            && content.right() <= width + slack
            && content.bottom() <= height + slack;
        let Some(edge) = Rect::from_xywh(0.0, 0.0, width, height).map(PathBuilder::from_rect)
        else {
            return Ok(());
        };
        let base = transform.pre_concat(pattern.transform());
        self.clipped(&clip, rule, false, |writer| {
            for row in rows {
                for column in columns.clone() {
                    let place = base.pre_translate(
                        cell.x() + column as f32 * width,
                        cell.y() + row as f32 * height,
                    );
                    if contained {
                        writer.children(pattern.root(), place)?;
                    } else if let Some(edge) = placed(&edge, place)? {
                        writer.clipped(&edge, usvg::FillRule::NonZero, false, |writer| {
                            writer.children(pattern.root(), place)
                        })?;
                    }
                }
            }
            Ok(())
        })
    }

    fn fill_mode(&mut self, rule: usvg::FillRule) {
        let mode = match rule {
            usvg::FillRule::EvenOdd => 1, // ALTERNATE
            usvg::FillRule::NonZero => 2, // WINDING
        };
        if self.fill_mode.replace(mode) != Some(mode) {
            self.record(19, &[mode]); // SETPOLYFILLMODE
        }
    }

    fn fill(
        &mut self,
        path: &Path,
        rule: usvg::FillRule,
        color: Option<Color>,
        transform: Transform,
    ) -> Result<(), String> {
        let (Some(color), Some(path)) = (color, placed(path, transform)?) else {
            return Ok(());
        };
        self.plus_path(&path, rule);
        self.plus(0x4014, 0x8000, &words(&[color.argb])); // FillPath, solid colour, object 0
        if self.brush != Some(color.gdi) {
            self.release_brush();
            self.record(39, &[BRUSH, 0, color.gdi, 0]); // CREATEBRUSHINDIRECT BS_SOLID
            self.record(37, &[BRUSH]);
            self.brush = Some(color.gdi);
        }
        self.fill_mode(rule);
        let bounds = path.bounds();
        self.gdi_path(&path);
        self.record(
            62, // FILLPATH with bounds in device pixels
            &[
                bounds.left().floor() as i32,
                bounds.top().floor() as i32,
                bounds.right().floor() as i32,
                bounds.bottom().floor() as i32,
            ],
        );
        Ok(())
    }

    /// Defines EMF+ path object 0 from a path in pixels.
    fn plus_path(&mut self, path: &Path, rule: usvg::FillRule) {
        let mut points: Vec<Point> = Vec::new();
        let mut kinds: Vec<u8> = Vec::new();
        for segment in cubic_segments(path) {
            match segment {
                Segment::Move(p) => {
                    points.push(p);
                    kinds.push(0);
                }
                Segment::Line(p) => {
                    points.push(p);
                    kinds.push(1);
                }
                Segment::Cubic(a, b, p) => {
                    points.extend([a, b, p]);
                    kinds.extend([3, 3, 3]);
                }
                Segment::Close => {
                    if let Some(last) = kinds.last_mut() {
                        *last |= 0x80;
                    }
                }
            }
        }
        let flags = match rule {
            usvg::FillRule::EvenOdd => 0,
            usvg::FillRule::NonZero => 0x2000,
        };
        let mut data = words(&[PLUS_VERSION, points.len() as u32, flags]);
        for point in &points {
            data.extend_from_slice(&point.x.to_le_bytes());
            data.extend_from_slice(&point.y.to_le_bytes());
        }
        data.extend_from_slice(&kinds);
        self.plus(0x4008, 0x0300, &data); // Object: path, id 0
    }

    /// Writes one GDI path bracket from a path in pixels.
    fn gdi_path(&mut self, path: &Path) {
        let point = |p: Point| {
            [
                (p.x * SCALE as f32).round() as i32,
                (p.y * SCALE as f32).round() as i32,
            ]
        };
        // (record type, points) runs: 6 POLYLINETO, 5 POLYBEZIERTO.
        let mut run: (i32, Vec<i32>) = (0, Vec::new());
        let flush = |writer: &mut Self, run: &mut (i32, Vec<i32>)| {
            if !run.1.is_empty() {
                let mut values = vec![0, 0, -1, -1, run.1.len() as i32 / 2];
                values.append(&mut run.1);
                writer.record(run.0, &values);
            }
        };
        self.record(59, &[]); // BEGINPATH
        for segment in cubic_segments(path) {
            let (kind, points) = match segment {
                Segment::Move(p) => {
                    flush(self, &mut run);
                    self.record(27, &point(p)); // MOVETOEX
                    continue;
                }
                Segment::Close => {
                    flush(self, &mut run);
                    self.record(61, &[]); // CLOSEFIGURE
                    continue;
                }
                Segment::Line(p) => (6, vec![p]),
                Segment::Cubic(a, b, p) => (5, vec![a, b, p]),
            };
            if run.0 != kind {
                flush(self, &mut run);
                run.0 = kind;
            }
            run.1.extend(points.into_iter().flat_map(point));
        }
        flush(self, &mut run);
        self.record(60, &[]); // ENDPATH
    }
}

enum Segment {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Close,
}

/// Path segments with quadratic curves raised to cubic ones.
fn cubic_segments(path: &Path) -> Vec<Segment> {
    let mut last = Point::zero();
    path.segments()
        .map(|segment| {
            let (segment, end) = match segment {
                PathSegment::MoveTo(p) => (Segment::Move(p), p),
                PathSegment::LineTo(p) => (Segment::Line(p), p),
                PathSegment::QuadTo(c, p) => {
                    let third = |from: Point| {
                        Point::from_xy(
                            from.x + 2.0 / 3.0 * (c.x - from.x),
                            from.y + 2.0 / 3.0 * (c.y - from.y),
                        )
                    };
                    (Segment::Cubic(third(last), third(p), p), p)
                }
                PathSegment::CubicTo(a, b, p) => (Segment::Cubic(a, b, p), p),
                PathSegment::Close => (Segment::Close, last),
            };
            last = end;
            segment
        })
        .collect()
}

/// The path in pixels, or `None` when the transform leaves nothing to draw.
fn placed(path: &Path, transform: Transform) -> Result<Option<Path>, String> {
    let Some(path) = path.clone().transform(transform) else {
        return Ok(None);
    };
    let bounds = path.bounds();
    if [bounds.left(), bounds.top(), bounds.right(), bounds.bottom()]
        .iter()
        .any(|value| !value.is_finite() || value.abs() > LIMIT)
    {
        return Err("coordinates outside the EMF range".into());
    }
    Ok(Some(path))
}

/// What a fill or stroke is painted with.
#[derive(Clone, Copy)]
enum Ink<'a> {
    Solid(Option<Color>),
    Pattern(&'a usvg::Pattern),
}

fn ink<'a>(paint: &'a usvg::Paint, opacity: f32, role: &str) -> Result<Ink<'a>, String> {
    match paint {
        usvg::Paint::Pattern(_) if opacity != 1.0 => Err(format!("a translucent pattern {role}")),
        usvg::Paint::Pattern(pattern) => Ok(Ink::Pattern(pattern)),
        _ => solid(paint, opacity, role).map(Ink::Solid),
    }
}

/// Indices of the tiles of size `size` starting at `origin` that cover `from..to`.
fn tile_range(from: f32, to: f32, origin: f32, size: f32) -> std::ops::Range<i64> {
    let first = ((from - origin) / size).floor();
    let end = ((to - origin) / size).ceil().max(first + 1.0);
    first as i64..end as i64
}

/// `None` is a fully transparent paint, which draws nothing.
fn solid(paint: &usvg::Paint, opacity: f32, role: &str) -> Result<Option<Color>, String> {
    let usvg::Paint::Color(color) = paint else {
        return Err(format!("a gradient {role}"));
    };
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    let (red, green, blue) = (
        u32::from(color.red),
        u32::from(color.green),
        u32::from(color.blue),
    );
    Ok((alpha > 0).then_some(Color {
        gdi: (red | green << 8 | blue << 16) as i32,
        argb: alpha << 24 | red << 16 | green << 8 | blue,
    }))
}

/// The outline a stroke covers, in the path's own coordinates.
fn stroke_outline(path: &Path, stroke: &usvg::Stroke, transform: Transform) -> Option<Path> {
    let mut pen = stroke.to_tiny_skia();
    let resolution =
        PathStroker::compute_resolution_scale(&transform.post_scale(SCALE as f32, SCALE as f32));
    let dashed = match pen.dash.take() {
        Some(dash) => path.dash(&dash, resolution),
        None => Some(path.clone()),
    };
    dashed.and_then(|line| line.stroke(&pen, resolution))
}

/// Collects the group's shapes in pixels into `union` when all of them are plain
/// non-zero fills of one opaque colour without strokes; otherwise returns false.
fn same_fill<'a>(
    group: &'a usvg::Group,
    transform: Transform,
    union: &mut PathBuilder,
    paint: &mut Option<&'a usvg::Paint>,
) -> Result<bool, String> {
    for node in group.children() {
        let inner = match node {
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                let Some(fill) = path.fill() else {
                    return Ok(false);
                };
                let same = match (fill.paint(), *paint) {
                    (usvg::Paint::Color(_), None) => true,
                    (usvg::Paint::Color(a), Some(usvg::Paint::Color(b))) => a == b,
                    _ => false,
                };
                if !same
                    || path.stroke().is_some()
                    || fill.opacity().get() != 1.0
                    || fill.rule() != usvg::FillRule::NonZero
                {
                    return Ok(false);
                }
                *paint = Some(fill.paint());
                if let Some(shape) = placed(path.data(), transform)? {
                    union.push_path(&shape);
                }
                continue;
            }
            usvg::Node::Group(inner) => inner,
            usvg::Node::Text(text) => text.flattened(),
            usvg::Node::Image(_) => return Ok(false),
        };
        if inner.opacity().get() != 1.0
            || inner.blend_mode() != usvg::BlendMode::Normal
            || inner.clip_path().is_some()
            || inner.mask().is_some()
            || !inner.filters().is_empty()
            || !same_fill(inner, transform.pre_concat(inner.transform()), union, paint)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether the group draws exactly one path, possibly through plain groups.
fn holds_one_path(group: &usvg::Group) -> bool {
    match group.children() {
        [usvg::Node::Path(_)] => true,
        [usvg::Node::Group(inner)] => {
            inner.opacity().get() == 1.0
                && inner.blend_mode() == usvg::BlendMode::Normal
                && inner.mask().is_none()
                && inner.filters().is_empty()
                && holds_one_path(inner)
        }
        [usvg::Node::Text(text)] => holds_one_path(text.flattened()),
        _ => false,
    }
}

/// The filled shapes of a clip path; this writer expresses only a single one.
fn clip_shapes(
    group: &usvg::Group,
    transform: Transform,
    shapes: &mut Vec<(Path, usvg::FillRule, Transform)>,
) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Path(path) => {
                if let (true, Some(fill)) = (path.is_visible(), path.fill()) {
                    shapes.push((path.data().clone(), fill.rule(), transform));
                }
            }
            usvg::Node::Group(group) => {
                if group.clip_path().is_some() {
                    return Err("a nested clip path".into());
                }
                clip_shapes(group, transform.pre_concat(group.transform()), shapes)?;
            }
            usvg::Node::Text(text) => clip_shapes(
                text.flattened(),
                transform.pre_concat(text.flattened().transform()),
                shapes,
            )?,
            usvg::Node::Image(_) => return Err("an image in a clip path".into()),
        }
    }
    Ok(())
}
