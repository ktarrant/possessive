use glam::{IVec2, Vec2};
use rand::SeedableRng;
use rand::Rng;
use rand_pcg::Pcg64Mcg;

use super::grid::{Grid, clamp};
use super::masks::Mask;
use super::noise::{Layer, LayerKind, layer_value};
use super::placement::{place_pack_near, scatter_forest, PackSpec};
use super::rng::RngSeq;
use super::river::carve_river;

use super::template::{MapTemplate, Symmetry};

pub struct GeneratedMap {
    pub size: IVec2,
    pub height: Grid<f32>,
    pub passable: Mask,
    pub water: Mask,
    pub forest: Vec<IVec2>,
    pub gold: Vec<IVec2>,
    pub berries: Vec<IVec2>,
    pub player_starts: Vec<IVec2>,
}

pub fn generate_map(tpl: &MapTemplate, seed: u64) -> GeneratedMap {
    let size = IVec2::new(tpl.size.0, tpl.size.1);
    let mut height = Grid::<f32>::new(size.x, size.y);
    let mut passable = Mask::new(size.x, size.y);
    let mut water = Mask::new(size.x, size.y);
    let mut gold = Vec::new();
    let mut berries = Vec::new();
    let mut forest = Vec::new();

    let seq = RngSeq::new(seed);

    // Phase: elevation
    {
        let mut rng = seq.for_phase(1);
        let center = size.as_vec2() / 2.0;
        let center = size.as_vec2() / 2.0;
        for y in 0..size.y {
            for x in 0..size.x {
                let p = glam::IVec2::new(x, y);
                let d = p.as_vec2().distance(center);
                let base = tpl.elevation.base_level - (d / tpl.elevation.falloff).powf(1.2);
                height.set(x, y, base);
            }
        }
        // layers
        for (i, l) in tpl.elevation.layers.iter().enumerate() {
            let layer = super::noise::Layer {
                freq: l.freq as f64,
                amp: l.amp,
                octaves: l.octaves as usize,
                kind: if l.ridged { super::noise::LayerKind::Ridged } else { super::noise::LayerKind::Fbm },
            };
            for y in 0..size.y {
                for x in 0..size.x {
                    let v = layer_value((seed as u32).wrapping_add(i as u32), x as f64, y as f64, &layer);
                    let cur = *height.get(x, y);
                    height.set(x, y, cur + v);
                }
            }
        }
        // mark initial passable (no cliffs > threshold)
        let mut max_slope: f32 = 0.0;
        for p in height.iter_xy() {
            let s = local_slope(&height, p);
            max_slope = max_slope.max(s);
        }
        for p in height.iter_xy() {
            let s = local_slope(&height, p);
            passable.set(p.x, p.y, s < max_slope * 0.6);
        }
        // Optional river (one diagonal) for demo
        if tpl.water {
            let src = IVec2::new((size.x as f32*0.2) as i32, (size.y as f32*0.2) as i32);
            let snk = IVec2::new((size.x as f32*0.85) as i32, (size.y as f32*0.8) as i32);
            carve_river(&mut height, &mut water, src, snk, 3.0);
            // water is unpassable
            for p in water.iter_true() { passable.set(p.x, p.y, false); }
        }
    }

    // Phase: player starts (symmetric)
    let player_starts = {
        let mut rng = seq.for_phase(2);
        place_symmetric_starts(&mut rng, size, &passable, &tpl.player_start_rules)
    };

    // Flatten small plateaus around starts
    for &s in &player_starts {
        flatten_plateau(&mut height, s, tpl.player_start_rules.plateau_radius, 0.7);
    }

    // Phase: per-player fairness packs
    {
        let mut rng = seq.for_phase(3);
        let center_excl = 10;
        let allowed = passable.and_not(&water);
        for &s in &player_starts {
            for _ in 0..tpl.fairness.primary_gold {
                let _ok = place_pack_near(&mut rng, &passable, &allowed, s, 18, 28, PackSpec::Gold { piles: 1 }, &mut gold, &mut berries);
                let _ok2 = place_pack_near(&mut rng, &passable, &allowed, s, 12, 22, PackSpec::Berries { bushes: 6 }, &mut gold, &mut berries);
            }
            if tpl.fairness.secondary_gold > 0 {
                let _ok = place_pack_near(&mut rng, &passable, &allowed, s, 26, 42, PackSpec::Gold { piles: 1 }, &mut gold, &mut berries);
            }
            // straggler trees near TC (scatter one-ring)
        }
        // Forest fill (blue-noise)
        let forest_allowed = passable.and_not(&water);
        let mut rng = seq.for_phase(3); // or reuse the phase you use for features
        let forest = scatter_forest(&mut rng, size, &forest_allowed, (1.0 / tpl.densities.forest_density).max(4.0), 30);
        for t in &forest { passable.set(t.x, t.y, false); }
    }

    GeneratedMap { size, height, passable, water, forest, gold, berries, player_starts }
}

fn local_slope(h: &Grid<f32>, p: glam::IVec2) -> f32 {
    let mut maxd: f32 = 0.0;
    let v = *h.get(p.x, p.y);
    for dy in -1..=1 { for dx in -1..=1 {
        if dx==0 && dy==0 { continue; }
        let q = glam::IVec2::new(p.x+dx, p.y+dy);
        if q.x<0 || q.y<0 || q.x>=h.w || q.y>=h.h { continue; }
        maxd = maxd.max((v - *h.get(q.x, q.y)).abs());
    }}
    maxd
}

fn flatten_plateau(h: &mut Grid<f32>, c: IVec2, r: i32, blend: f32) {
    let base = *h.get(c.x, c.y);
    for y in (c.y-r)..=(c.y+r) {
        for x in (c.x-r)..=(c.x+r) {
            if x<0 || y<0 || x>=h.w || y>=h.h { continue; }
            let d = ((x-c.x).pow(2) + (y-c.y).pow(2)) as f32;
            if d <= (r*r) as f32 {
                let cur = *h.get(x,y);
                h.set(x,y, cur*(1.0-blend) + base*blend);
            }
        }
    }
}

use super::template::PlayerStartRules;

fn place_symmetric_starts(
    rng: &mut Pcg64Mcg,
    size: IVec2,
    passable: &Mask,
    rules: &PlayerStartRules,
) -> Vec<IVec2> {
    let n = match rules.symmetry {
        Symmetry::None => 2,
        Symmetry::Rotational(k) => k.max(2) as usize,
    };
    let center = size.as_vec2()/2.0;
    let radius = rules.min_to_center.max((size.x.min(size.y) as f32 * 0.28) as i32) as f32;

    let base_angle = rng.gen::<f32>() * std::f32::consts::TAU; // <-- now compiles
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let ang = base_angle + i as f32 * (std::f32::consts::TAU / n as f32);
        let mut p = center + Vec2::new(ang.cos()*radius, ang.sin()*radius);
        p += Vec2::new(
            rng.gen_range(-(rules.jitter as i32)..=rules.jitter) as f32,
            rng.gen_range(-(rules.jitter as i32)..=rules.jitter) as f32
        );
        let mut g = IVec2::new(p.x.round() as i32, p.y.round() as i32);
        g = clamp(g, size.x, size.y);
        // slide to nearest passable if needed
        if !passable.get(g.x, g.y) {
            for r in 1..32 {
                let mut found = None;
                'srch: for dy in -r..=r { for dx in -r..=r {
                    let q = IVec2::new(g.x+dx, g.y+dy);
                    if q.x<0||q.y<0||q.x>=size.x||q.y>=size.y { continue; }
                    if passable.get(q.x,q.y) { found = Some(q); break 'srch; }
                } }
                if let Some(q) = found { g=q; break; }
            }
        }
        out.push(g);
    }
    out
}
