use glam::IVec2;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_pcg::Pcg64Mcg;

pub fn poisson_sample(
    bounds: IVec2,
    r: f32,
    k: usize,
    rng: &mut Pcg64Mcg,
    valid: impl Fn(IVec2) -> bool,
) -> Vec<IVec2> {
    let cell = r / f32::sqrt(2.0);
    let gw = (bounds.x as f32 / cell).ceil() as i32;
    let gh = (bounds.y as f32 / cell).ceil() as i32;
    let mut grid: Vec<Option<IVec2>> = vec![None; (gw*gh) as usize];
    let mut samples = Vec::new();
    let mut active = Vec::new();

    let s0 = IVec2::new(rng.gen_range(0..bounds.x), rng.gen_range(0..bounds.y));
    if !valid(s0) { return samples; }
    insert(&mut grid, cell, gw, s0);
    samples.push(s0);
    active.push(s0);

    while let Some(&s) = active.choose(rng) {
        let mut found = false;
        for _ in 0..k {
            let cand = annulus_point(s, r, 2.0*r, rng, bounds);
            if valid(cand) && far_from_neighbors(&grid, cell, gw, cand, r) {
                insert(&mut grid, cell, gw, cand);
                samples.push(cand);
                active.push(cand);
                found = true; break;
            }
        }
        if !found {
            if let Some(i) = active.iter().position(|p| *p == s) { active.swap_remove(i); }
        }
    }
    samples
}

fn insert(grid: &mut [Option<IVec2>], cell: f32, gw: i32, p: IVec2) {
    let gi = grid_idx(cell, gw, p);
    grid[gi] = Some(p);
}
fn grid_idx(cell: f32, gw: i32, p: IVec2) -> usize {
    let gx = (p.x as f32 / cell).floor() as i32;
    let gy = (p.y as f32 / cell).floor() as i32;
    (gy*gw + gx) as usize
}
fn far_from_neighbors(grid: &[Option<IVec2>], cell: f32, gw: i32, p: IVec2, r: f32) -> bool {
    let gx = (p.x as f32 / cell).floor() as i32;
    let gy = (p.y as f32 / cell).floor() as i32;
    for ny in (gy-2)..=(gy+2) {
        for nx in (gx-2)..=(gx+2) {
            if nx<0 || ny<0 { continue; }
            let idx = (ny*gw + nx) as usize;
            if let Some(q) = grid.get(idx).and_then(|o| *o) {
                if q.as_vec2().distance(p.as_vec2()) < r { return false; }
            }
        }
    }
    true
}
fn annulus_point(center: IVec2, rmin: f32, rmax: f32, rng: &mut Pcg64Mcg, bounds: IVec2) -> IVec2 {
    let a = rng.gen::<f32>() * std::f32::consts::TAU;
    let r = f32::sqrt(rng.gen::<f32>()*(rmax*rmax - rmin*rmin) + rmin*rmin);
    let p = center.as_vec2() + glam::vec2(a.cos()*r, a.sin()*r);
    IVec2::new(p.x.round() as i32, p.y.round() as i32).clamp(IVec2::ZERO, bounds-1)
}
