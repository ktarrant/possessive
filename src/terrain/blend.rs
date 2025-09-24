use super::grid::Grid;
use super::template::MapTemplate;
use glam::IVec2;
use super::debug_png::write_terrain_classes;

// Keep the same ids as landscape.rs
pub const TERRAIN_GRASSLAND: u8 = 0;
pub const TERRAIN_FOREST:    u8 = 1;
pub const TERRAIN_WATER:     u8 = 2;
pub const TERRAIN_MOUNTAIN:  u8 = 3;

#[derive(Clone, Copy)]
pub struct BlendSettings {
    /// Number of blur→relabel iterations (2–4 is typical).
    pub iterations: usize,
    /// Per-class blur radii (tiles). e.g., forest a bit fuzzier than water.
    pub radii: [i32; 4],
    /// Bias to keep the current label (0.0–1.0). ~0.25 keeps interiors stable.
    pub inertia: f32,
    /// If true, only blend pixels that sit on a class boundary.
    pub boundary_only: bool,
}

impl Default for BlendSettings {
    fn default() -> Self {
        Self {
            iterations: 3,
            radii: [2, 3, 2, 3], // grass, forest, water, mountain
            inertia: 0.25,
            boundary_only: true,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FractalSettings {
    pub iterations: usize,      // 1–3; more = stronger, slower
    pub radii: [i32; 4],        // blur radii per class (grass, forest, water, mountain)
    pub inertia: f32,           // 0..1 bias to keep current label
    pub boundary_only: bool,    // only modify boundary pixels
    pub warp_amp: f32,          // max displacement in pixels (e.g., 3.5)
    pub warp_freq: f32,         // base frequency in cycles per tile (e.g., 1/24 ≈ 0.0417)
    pub warp_octaves: u32,      // 2–4 usually enough
    pub warp_gain: f32,         // amplitude falloff per octave (e.g., 0.5)
    pub warp_lacunarity: f32,   // frequency growth per octave (e.g., 2.0)
    pub seed: u32,              // randomization, deterministic
}

impl Default for FractalSettings {
    fn default() -> Self {
        Self {
            iterations: 2,
            radii: [2, 3, 2, 3],
            inertia: 0.25,
            boundary_only: true,
            warp_amp: 4.0,
            warp_freq: 1.0 / 24.0,
            warp_octaves: 3,
            warp_gain: 0.5,
            warp_lacunarity: 2.0,
            seed: 1337,
        }
    }
}


// ---- helpers ----------------------------------------------------------------

#[inline] fn idx(w: i32, x: i32, y: i32) -> usize { (y * w + x) as usize }

fn circle_mask(size: IVec2, c: IVec2, r: i32) -> Vec<u8> {
    let (w,h) = (size.x, size.y);
    let r2 = (r.max(0) * r.max(0)) as i32;
    let mut m = vec![0u8; (w*h) as usize];
    let xmin = (c.x - r).max(0);
    let xmax = (c.x + r).min(w-1);
    let ymin = (c.y - r).max(0);
    let ymax = (c.y + r).min(h-1);
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            let dx = x - c.x; let dy = y - c.y;
            if dx*dx + dy*dy <= r2 { m[idx(w,x,y)] = 1; }
        }
    }
    m
}

fn build_locked_mask(
    tpl: &MapTemplate,
    base_centers: &[IVec2],
    shrines: &[IVec2],
) -> Vec<u8> {
    let size = IVec2::new(tpl.size.0, tpl.size.1);
    let (w,h) = (size.x, size.y);
    let mut locked = vec![0u8; (w*h) as usize];

    // base rings
    let br = tpl.player_spawns.base_radius.max(1);
    for &c in base_centers {
        let m = circle_mask(size, c, br);
        for i in 0..m.len() { if m[i]!=0 { locked[i]=1; } }
    }
    // shrine rings
    let sr = tpl.terrain.shrine_grass_radius.max(0);
    for &s in shrines {
        let m = circle_mask(size, s, sr);
        for i in 0..m.len() { if m[i]!=0 { locked[i]=1; } }
    }
    locked
}

#[inline]
fn is_boundary(classes: &Grid<u8>, x: i32, y: i32) -> bool {
    let c = *classes.get(x,y);
    // 4-neighborhood is usually enough; use 8-neighb if you want more activity
    if x > 0               && *classes.get(x-1,y) != c { return true; }
    if x < classes.w-1     && *classes.get(x+1,y) != c { return true; }
    if y > 0               && *classes.get(x,y-1) != c { return true; }
    if y < classes.h-1     && *classes.get(x,y+1) != c { return true; }
    false
}

// Box blur via separable passes using prefix sums (fast and artifact-free for our use).
fn box_blur(w: i32, h: i32, src: &[f32], radius: i32, dst: &mut [f32]) {
    if radius <= 0 {
        dst.copy_from_slice(src);
        return;
    }
    let r = radius as usize;
    let mut tmp = vec![0f32; src.len()];

    // horizontal
    for y in 0..h as usize {
        let row = &src[(y*(w as usize))..((y+1)*(w as usize))];
        let mut prefix = vec![0f32; w as usize + 1];
        for x in 0..w as usize { prefix[x+1] = prefix[x] + row[x]; }
        for x in 0..w as usize {
            let xl = x.saturating_sub(r);
            let xr = (x + r).min(w as usize - 1);
            let count = (xr - xl + 1) as f32;
            let sum = prefix[xr+1] - prefix[xl];
            tmp[y*(w as usize) + x] = sum / count;
        }
    }

    // vertical
    for x in 0..w as usize {
        let mut prefix = vec![0f32; h as usize + 1];
        for y in 0..h as usize { prefix[y+1] = prefix[y] + tmp[y*(w as usize) + x]; }
        for y in 0..h as usize {
            let yl = y.saturating_sub(r);
            let yr = (y + r).min(h as usize - 1);
            let count = (yr - yl + 1) as f32;
            let sum = prefix[yr+1] - prefix[yl];
            dst[y*(w as usize) + x] = sum / count;
        }
    }
}

#[inline] fn fade(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }
#[inline] fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

// Hash a lattice point (xi, yi) + seed into [0,1)
fn hash01_lattice(xi: i32, yi: i32, seed: u32) -> f32 {
    let mut v = (xi as u32).wrapping_mul(0x9E3779B1)
        ^ (yi as u32).wrapping_mul(0x85EBCA77)
        ^ seed.wrapping_mul(0xC2B2AE3D);
    v ^= v >> 16; v = v.wrapping_mul(0x7feb352d);
    v ^= v >> 15; v = v.wrapping_mul(0x846ca68b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32) // [0,1)
}

// Value noise with smooth bilinear interpolation, output in [-1,1]
fn value_noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32; let y0 = y.floor() as i32;
    let tx = x - x0 as f32;    let ty = y - y0 as f32;
    let n00 = hash01_lattice(x0,     y0,     seed);
    let n10 = hash01_lattice(x0 + 1, y0,     seed);
    let n01 = hash01_lattice(x0,     y0 + 1, seed);
    let n11 = hash01_lattice(x0 + 1, y0 + 1, seed);
    let ux = fade(tx); let uy = fade(ty);
    let a = lerp(n00, n10, ux);
    let b = lerp(n01, n11, ux);
    let v = lerp(a, b, uy);
    v * 2.0 - 1.0 // [-1,1]
}

// Fractal Brownian Motion
fn fbm_2d(mut x: f32, mut y: f32, seed: u32, octaves: u32, gain: f32, lacunarity: f32) -> f32 {
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        let v = value_noise_2d(x * freq, y * freq, seed.wrapping_add(o));
        sum += v * amp;
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    if norm > 0.0 { sum / norm } else { 0.0 }
}

// Bilinear sample from a scalar buffer
fn sample_scalar(buf: &[f32], w: i32, h: i32, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let x0 = x.floor() as i32; let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min(w - 1); let y1 = (y0 + 1).min(h - 1);
    let tx = x - x0 as f32; let ty = y - y0 as f32;
    let i00 = idx(w, x0, y0); let i10 = idx(w, x1, y0);
    let i01 = idx(w, x0, y1); let i11 = idx(w, x1, y1);
    let a = lerp(buf[i00], buf[i10], tx);
    let b = lerp(buf[i01], buf[i11], tx);
    lerp(a, b, ty)
}

// ---- API --------------------------------------------------------------------

/// Phase 4: Blend sharp borders into natural transitions.
/// Returns a new classes grid and writes `phase4_blended.png`.
pub fn blend_terrain_and_save(
    tpl: &MapTemplate,
    base_centers: &[IVec2],
    shrines: &[IVec2],
    classes_in: &Grid<u8>,
    settings: BlendSettings,
    out_path: &str,
) -> Grid<u8> {
    let (w,h) = (classes_in.w, classes_in.h);
    let total = (w*h) as usize;

    // Hard pins we must preserve as grass.
    let locked = build_locked_mask(tpl, base_centers, shrines);

    // Work buffers
    let mut classes = classes_in.clone();
    let mut next = Grid::<u8>::new(w,h);

    // One-hot channels and blur buffers
    let mut chan = [vec![0f32; total], vec![0f32; total], vec![0f32; total], vec![0f32; total]];
    let mut blurred = [vec![0f32; total], vec![0f32; total], vec![0f32; total], vec![0f32; total]];

    for _it in 0..settings.iterations {
        // Build one-hot per class (skip locked—keep them pure grass contribution)
        for k in 0..4 { chan[k].fill(0.0); }
        for y in 0..h {
            for x in 0..w {
                let i = idx(w, x, y);
                let k = if locked[i] != 0 { TERRAIN_GRASSLAND } else { *classes.get(x,y) };
                chan[k as usize][i] = 1.0;
            }
        }

        // Blur each class channel with its own radius
        for k in 0..4 {
            box_blur(w, h, &chan[k], settings.radii[k], &mut blurred[k]);
        }

        // Reassign labels (argmax), with inertia to keep the current class
        for y in 0..h {
            for x in 0..w {
                let i = idx(w,x,y);

                // Respect hard pins
                if locked[i] != 0 {
                    next.set(x,y, TERRAIN_GRASSLAND);
                    continue;
                }

                if settings.boundary_only && !is_boundary(&classes, x, y) {
                    // Keep interior pixels untouched
                    next.set(x,y, *classes.get(x,y));
                    continue;
                }

                let cur = *classes.get(x,y) as usize;

                // Score = blurred affinity + inertia if same as current class
                let mut best_k = 0usize;
                let mut best_s = f32::NEG_INFINITY;
                for k in 0..4 {
                    let mut s = blurred[k][i];
                    if k == cur { s += settings.inertia; }
                    if s > best_s { best_s = s; best_k = k; }
                }
                next.set(x,y, best_k as u8);
            }
        }

        // swap
        classes = next.clone();
    }

    // Defensive: ensure locked stay grass
    for y in 0..h { for x in 0..w {
        if locked[idx(w,x,y)] != 0 { classes.set(x,y, TERRAIN_GRASSLAND); }
    }}

    // Save debug image
    const PALETTE: [[u8;4];4] = [
        [110,180,110,255], // grass
        [ 34,139, 34,255], // forest
        [ 64,120,255,255], // water
        [150,150,150,255], // mountain
    ];
    write_terrain_classes(out_path, &classes, &PALETTE);

    classes
}

pub fn blend_fractal_and_save(
    tpl: &MapTemplate,
    base_centers: &[IVec2],
    shrines: &[IVec2],
    classes_in: &Grid<u8>,
    settings: FractalSettings,
    out_path: &str,
) -> Grid<u8> {
    let (w,h) = (classes_in.w, classes_in.h);
    let total = (w*h) as usize;

    // Hard grass rings to preserve
    let locked = build_locked_mask(tpl, base_centers, shrines);

    // Work grids
    let mut classes = classes_in.clone();
    let mut next = Grid::<u8>::new(w,h);

    // One-hot and blurred channels
    let mut chan = [vec![0f32; total], vec![0f32; total], vec![0f32; total], vec![0f32; total]];
    let mut blurred = [vec![0f32; total], vec![0f32; total], vec![0f32; total], vec![0f32; total]];

    let seed_x = settings.seed.wrapping_add(0xB5297A4D);
    let seed_y = settings.seed.wrapping_add(0x68E31DA4);

    for _ in 0..settings.iterations {
        // Rebuild one-hot with locked areas forced to grass
        for k in 0..4 { chan[k].fill(0.0); }
        for y in 0..h {
            for x in 0..w {
                let i = idx(w,x,y);
                let k = if locked[i]!=0 { super::blend::TERRAIN_GRASSLAND } else { *classes.get(x,y) };
                chan[k as usize][i] = 1.0;
            }
        }

        // Blur each channel with its radius
        for k in 0..4 {
            box_blur(w, h, &chan[k], settings.radii[k], &mut blurred[k]);
        }

        // Domain-warped relabel
        for y in 0..h {
            for x in 0..w {
                let i = idx(w,x,y);

                if locked[i] != 0 {
                    next.set(x,y, super::blend::TERRAIN_GRASSLAND);
                    continue;
                }
                if settings.boundary_only && !is_boundary(&classes, x, y) {
                    next.set(x,y, *classes.get(x,y));
                    continue;
                }

                // Displacement field via fBm
                let xf = x as f32 * settings.warp_freq;
                let yf = y as f32 * settings.warp_freq;
                let dx = settings.warp_amp * fbm_2d(xf, yf, seed_x, settings.warp_octaves, settings.warp_gain, settings.warp_lacunarity);
                let dy = settings.warp_amp * fbm_2d(xf, yf, seed_y, settings.warp_octaves, settings.warp_gain, settings.warp_lacunarity);

                let cur = *classes.get(x,y) as usize;

                let mut best_k = 0usize;
                let mut best_s = f32::NEG_INFINITY;
                for k in 0..4 {
                    let s = sample_scalar(&blurred[k], w, h, x as f32 + dx, y as f32 + dy)
                        + if k == cur { settings.inertia } else { 0.0 };
                    if s > best_s { best_s = s; best_k = k; }
                }
                next.set(x,y, best_k as u8);
            }
        }

        classes = next.clone();
    }

    // Keep locked areas grass, defensively
    for y in 0..h { for x in 0..w {
        if locked[idx(w,x,y)]!=0 { classes.set(x,y, super::blend::TERRAIN_GRASSLAND); }
    }}

    // Save preview
    const PALETTE: [[u8;4];4] = [
        [110,180,110,255],
        [ 34,139, 34,255],
        [ 64,120,255,255],
        [150,150,150,255],
    ];
    write_terrain_classes(out_path, &classes, &PALETTE);

    classes
}
