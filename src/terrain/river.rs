use super::grid::Grid;
use glam::IVec2;
use pathfinding::prelude::astar;

pub fn carve_river(
    height: &mut Grid<f32>,
    water: &mut super::masks::Mask,
    source: IVec2,
    sink: IVec2,
    width: f32,
) {
    let w = height.w; let h = height.h;
    let neigh = |p: IVec2| {
        let mut v = Vec::new();
        for dy in -1..=1 { for dx in -1..=1 {
            if dx==0 && dy==0 { continue; }
            let q = IVec2::new(p.x+dx, p.y+dy);
            if q.x>=0 && q.y>=0 && q.x<w && q.y<h { v.push(q); }
        }}
        v
    };
    // integer cost
    let cost = |a: IVec2, b: IVec2| -> u32 {
        let dh = height.get(b.x, b.y) - height.get(a.x, a.y);
        let uphill_penalty = if dh > 0.0 { (dh * 80.0) as u32 } else { 0 };
        10 + uphill_penalty  // base + penalty
    };

    if let Some((path, _)) = astar(
        &source,
        |p| {
            let src = *p;
            neigh(src)
                .into_iter()
                .map(|q| (q, cost(src, q)))
                .collect::<Vec<_>>() // own it, no lifetime issues
        },
        |p| (p.x - sink.x).abs() as u32 + (p.y - sink.y).abs() as u32,
        |p| *p == sink,
    ) {
        for &p in &path {
            let bed = height.get(p.x, p.y) - 3.0;
            *height.get_mut(p.x, p.y) = bed;
            brush_circle(height, water, p, width);
        }
    }
}

fn brush_circle(height: &mut Grid<f32>, water: &mut super::masks::Mask, p: IVec2, r: f32) {
    let ir = r.ceil() as i32;
    for y in (p.y-ir)..=(p.y+ir) {
        for x in (p.x-ir)..=(p.x+ir) {
            if x<0 || y<0 || x>=height.w || y>=height.h { continue; }
            let dx = (x - p.x) as f32;
            let dy = (y - p.y) as f32;
            let d2 = dx*dx + dy*dy;
            if d2 <= r*r {
                let fall = (r*r - d2).sqrt() * 0.15;
                *height.get_mut(x,y) -= fall;
                water.set(x,y, true);
            }
        }
    }
}
