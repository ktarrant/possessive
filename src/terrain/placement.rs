use glam::{IVec2, Vec2};
use rand::Rng;
use rand_pcg::Pcg64Mcg;

use super::{masks::Mask, poisson::poisson_sample};

#[derive(Clone, Copy)]
pub enum PackSpec { Gold { piles: usize }, Berries { bushes: usize } }

pub fn place_pack_near(
    rng: &mut Pcg64Mcg,
    passable: &Mask,
    allowed: &Mask,
    start: IVec2,
    min_r: i32,
    max_r: i32,
    spec: PackSpec,
    out_gold: &mut Vec<IVec2>,
    out_berries: &mut Vec<IVec2>,
) -> bool {
    let mut tries = 64;
    while tries>0 {
        tries -= 1;
        let ang = rng.gen::<f32>() * std::f32::consts::TAU;
        let rad = rng.gen_range(min_r..=max_r) as f32;
        let p = start.as_vec2() + Vec2::new(ang.cos()*rad, ang.sin()*rad);
        let p = IVec2::new(p.x.round() as i32, p.y.round() as i32);
        if p.x<0 || p.y<0 || p.x>=passable.w || p.y>=passable.h { continue; }
        if !allowed.get(p.x, p.y) || !passable.get(p.x, p.y) { continue; }
        match spec {
            PackSpec::Gold { piles } => {
                let pts = cluster_circle(p, piles as i32, 2);
                for c in pts { out_gold.push(c); }
                return true;
            }
            PackSpec::Berries { bushes } => {
                let pts = cluster_circle(p, bushes as i32, 2);
                for c in pts { out_berries.push(c); }
                return true;
            }
        }
    }
    false
}

fn cluster_circle(center: IVec2, count: i32, r: i32) -> Vec<IVec2> {
    let mut v = Vec::new();
    let mut added = 0;
    let mut y = -r;
    while y<=r && added < count {
        let mut x = -r;
        while x<=r && added < count {
            if x*x + y*y <= r*r {
                v.push(center + IVec2::new(x,y));
                added += 1;
            }
            x+=1;
        }
        y+=1;
    }
    v
}

pub fn scatter_forest(
    rng: &mut Pcg64Mcg,
    bounds: IVec2,
    allowed: &Mask,
    min_radius: f32,
    k: usize,
) -> Vec<IVec2> {
    poisson_sample(bounds, min_radius, k, rng, |p| allowed.get(p.x, p.y))
}
