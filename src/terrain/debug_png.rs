use image::{ImageBuffer, Rgba};
use glam::IVec2;
use super::grid::Grid;

/// Height-only grayscale for simple snapshots.
pub fn write_height_grayscale(path: &str, height: &Grid<f32>) {
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

    for y in 0..height.h {
        for x in 0..height.w {
            let v = (*height.get(x, y) - minv) / span;
            let g = (v * 255.0).clamp(0.0, 255.0) as u8;
            img.put_pixel(x as u32, y as u32, Rgba([g, g, g, 255]));
        }
    }

    img.save(path).expect("save png");
}

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
