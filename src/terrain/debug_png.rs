use image::{ImageBuffer, Rgba};
use glam::IVec2;
use super::grid::Grid;
use super::template::{MapTemplate, ObjectTypeRule};
use super::objects::PlacedObject;

/// Grayscale height with filled colored disks overlaid (e.g., base areas).
pub fn write_height_with_disks(
    path: &str,
    height: &Grid<f32>,
    disks: &[(IVec2, i32, [u8;3])], // (center, radius, rgb)
) {
    let w = height.w as u32; let h = height.h as u32;
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    // normalize to [0,255]
    let mut minv = f32::MAX; let mut maxv = f32::MIN;
    for y in 0..height.h {
        for x in 0..height.w {
            let v = *height.get(x, y);
            if v < minv { minv = v; }
            if v > maxv { maxv = v; }
        }
    }
    let span = (maxv - minv).max(1e-6);

    // base grayscale
    for y in 0..height.h {
        for x in 0..height.w {
            let v = (*height.get(x, y) - minv) / span;
            let g = (v * 255.0).clamp(0.0, 255.0) as u8;
            img.put_pixel(x as u32, y as u32, Rgba([g, g, g, 255]));
        }
    }

    // overlay filled disks
    for &(c, r, rgb) in disks {
        let ir = r.max(0);
        for y in (c.y - ir)..=(c.y + ir) {
            if y < 0 || y >= height.h { continue; }
            for x in (c.x - ir)..=(c.x + ir) {
                if x < 0 || x >= height.w { continue; }
                let dx = x - c.x;
                let dy = y - c.y;
                if dx*dx + dy*dy <= ir*ir {
                    img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], 255]));
                }
            }
        }
    }

    img.save(path).expect("save png");
}


// Simple Bresenham line drawer
fn draw_line(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, a: IVec2, b: IVec2, color: [u8;4]) {
    let (mut x0, mut y0) = (a.x, a.y);
    let (x1, y1) = (b.x, b.y);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0>=0 && y0>=0 && (x0 as u32) < img.width() && (y0 as u32) < img.height() {
            img.put_pixel(x0 as u32, y0 as u32, Rgba(color));
        }
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x0 += sx; }
        if e2 <= dx { err += dx; y0 += sy; }
    }
}

fn draw_disk(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, c: IVec2, r: i32, color: [u8;4]) {
    let r = r.max(0);
    for y in (c.y - r)..=(c.y + r) {
        if y < 0 || (y as u32) >= img.height() { continue; }
        for x in (c.x - r)..=(c.x + r) {
            if x < 0 || (x as u32) >= img.width() { continue; }
            let dx = x - c.x;
            let dy = y - c.y;
            if dx*dx + dy*dy <= r*r {
                img.put_pixel(x as u32, y as u32, Rgba(color));
            }
        }
    }
}

/// Height with overlays: base disks (e.g., red), shrine dots (e.g., cyan),
/// and ley line segments (e.g., blue/green).
pub fn write_height_with_overlays(
    path: &str,
    height: &Grid<f32>,
    base_disks: &[(IVec2, i32, [u8;4])],
    shrine_points: &[(IVec2, [u8;4])],
    ley_lines: &[(IVec2, IVec2, [u8;4])],
) {
    let w = height.w as u32; let h = height.h as u32;
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    // normalize height to grayscale
    let mut minv = f32::MAX; let mut maxv = f32::MIN;
    for y in 0..height.h { for x in 0..height.w {
        let v = *height.get(x,y);
        if v < minv { minv = v; }
        if v > maxv { maxv = v; }
    }}
    let span = (maxv - minv).max(1e-6);
    for y in 0..height.h { for x in 0..height.w {
        let v = (*height.get(x,y) - minv) / span;
        let g = (v * 255.0).clamp(0.0, 255.0) as u8;
        img.put_pixel(x as u32, y as u32, Rgba([g,g,g,255]));
    }}

    // ley lines first so points/disks can sit on top
    for &(a, b, color) in ley_lines {
        draw_line(&mut img, a, b, color);
    }

    // base disks
    for &(c, r, color) in base_disks {
        draw_disk(&mut img, c, r, color);
    }

    // shrines
    for &(p, color) in shrine_points {
        draw_disk(&mut img, p, 2, color); // tiny dot
    }

    img.save(path).expect("save png");
}

pub fn write_terrain_classes(
    path: &str,
    classes: &Grid<u8>,                 // 0..=3 terrain IDs
    palette: &[[u8; 4]; 4],             // RGBA for each class
) {
    let (w, h) = (classes.w as u32, classes.h as u32);
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for y in 0..classes.h {
        for x in 0..classes.w {
            let id = *classes.get(x, y).min(&3u8);
            let c = palette[id as usize];
            img.put_pixel(x as u32, y as u32, Rgba(c));
        }
    }
    img.save(path).expect("save png");
}

fn color_for_type(i: usize, t: &ObjectTypeRule) -> [u8;4] {
    let name = t.name.to_lowercase();
    if name.contains("tree")  { return [255, 220,   0, 255]; } // bright yellow
    if name.contains("bush")  { return [255, 120,   0, 255]; } // vivid orange
    if name.contains("cave")  { return [255,   0, 200, 255]; } // magenta
    if name.contains("rock")  { return [255,   0,   0, 255]; } // red
    if name.contains("ruin")  { return [  0, 255, 255, 255]; } // cyan

    // Fallback: stable pseudo-random color from index
    let mut x = (i as u32).wrapping_mul(0x9E3779B1);
    x ^= x >> 16; x = x.wrapping_mul(0x7FEB352D);
    x ^= x >> 15; x = x.wrapping_mul(0x846CA68B);
    x ^= x >> 16;
    // Spread bits into RGB-ish
    let r = (x & 0xFF) as u8;
    let g = ((x >> 8) & 0xFF) as u8;
    let b = ((x >> 16) & 0xFF) as u8;
    [r.saturating_add(40), g.saturating_add(40), b.saturating_add(40), 255]
}

/// Draw a filled disk into `img` with center `c` and radius `r` (pixels), colored `rgba`.
fn draw_filled_disk(img: &mut image::RgbaImage, c: IVec2, r: i32, rgba: [u8;4]) {
    if r <= 0 { return; }
    let (w, h) = img.dimensions();
    let (w, h) = (w as i32, h as i32);
    let rr = r * r;
    let xmin = (c.x - r).max(0);
    let xmax = (c.x + r).min(w - 1);
    let ymin = (c.y - r).max(0);
    let ymax = (c.y + r).min(h - 1);
    let px = Rgba(rgba);
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            let dx = x - c.x;
            let dy = y - c.y;
            if dx*dx + dy*dy <= rr {
                img.put_pixel(x as u32, y as u32, px);
            }
        }
    }
}

/// Write terrain classes as usual, then overlay object dots.
/// `palette` is the same one you pass to `write_terrain_classes`.
/// Objects are colored by type (Tree/Bush/Cave names matched case-insensitively),
/// otherwise a stable fallback color per type index.
///
/// Tip: we draw a small disk with radius = max(1, round(0.6 * object_type.radius))
pub fn write_terrain_with_objects(
    path: &str,
    classes: &Grid<u8>,
    palette: &[[u8;4];4],
    objects: &[PlacedObject],
    tpl: &MapTemplate,
) {
    // 1) Base terrain
    write_terrain_classes(path, classes, palette);

    // 2) Load and overlay
    if let Ok(dyn_img) = image::open(path) {
        let mut img = dyn_img.to_rgba8();

        // Precompute colors and draw radii for each type
        let mut type_colors: Vec<[u8;4]> = Vec::with_capacity(tpl.objects.types.len());
        let mut type_draw_r: Vec<i32> = Vec::with_capacity(tpl.objects.types.len());
        for (i, t) in tpl.objects.types.iter().enumerate() {
            type_colors.push(color_for_type(i, t));
            // smaller than the hard min-distance radius so dots don’t look huge
            let r = (t.radius as f32 * 0.6).round() as i32;
            type_draw_r.push(r.max(1));
        }

        for o in objects {
            let k = o.kind as usize;
            if k >= type_colors.len() { continue; }
            let color = type_colors[k];
            let rdraw = type_draw_r[k];
            draw_filled_disk(&mut img, o.pos, rdraw, color);

            // Optional: add a thin outline for visibility on light tiles
            // Uncomment if you want:
            // draw_filled_disk(&mut img, o.pos, (rdraw+1).min(rdraw+1), [0,0,0,80]);
        }

        let _ = img.save(path);
    }
}
