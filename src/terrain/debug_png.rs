use super::grid::Grid;
use super::masks::Mask;
use image::{ImageBuffer, Rgba};

pub fn write_height_rgb(path: &str, height: &Grid<f32>, passable: &Mask, water: &Mask) {
    let w = height.w as u32; let h = height.h as u32;
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    // normalize
    let mut minv = f32::MAX; let mut maxv = f32::MIN;
    for p in height.iter_xy() {
        let v = *height.get(p.x, p.y);
        minv = minv.min(v); maxv = maxv.max(v);
    }
    let span = (maxv - minv).max(1e-3);
    for y in 0..height.h {
        for x in 0..height.w {
            let v = (*height.get(x,y) - minv) / span;
            let b = (v*255.0) as u8;
            let mut px = [b,b,b,255];
            if water.get(x,y) { px = [30, 90, 200, 255]; }
            else if !passable.get(x,y) { px = [80, 80, 80, 255]; }
            img.put_pixel(x as u32, y as u32, Rgba(px));
        }
    }
    img.save(path).expect("save png");
}
