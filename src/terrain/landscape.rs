use glam::IVec2;
use super::grid::Grid;
use super::template::{MapTemplate, TerrainWeights, AreaSource};

// Terrain class ids
pub const TERRAIN_GRASSLAND: u8 = 0;
pub const TERRAIN_FOREST:    u8 = 1;
pub const TERRAIN_WATER:     u8 = 2;
pub const TERRAIN_MOUNTAIN:  u8 = 3;

// ---------------- tiny RNG (deterministic, no extra deps) ----------------
struct Rng64(u64);
impl Rng64 {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next_u32(&mut self) -> u32 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        (z ^ (z >> 31)) as u32
    }
    fn f01(&mut self) -> f32 { (self.next_u32() as f32) / (u32::MAX as f32) }
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 { // inclusive
        if hi <= lo { return lo; }
        lo + (self.next_u32() % ((hi - lo + 1) as u32)) as i32
    }
    fn choose_idx(&mut self, len: usize) -> usize {
        if len == 0 { 0 } else { (self.next_u32() as usize) % len }
    }
}

#[inline] fn idx(w: i32, x: i32, y: i32) -> usize { (y * w + x) as usize }

fn clamp_radius_to_bounds(size: IVec2, c: IVec2, r: i32) -> i32 {
    let rmax_x = (c.x).min(size.x - 1 - c.x);
    let rmax_y = (c.y).min(size.y - 1 - c.y);
    r.min(rmax_x).min(rmax_y).max(0)
}

#[inline] fn weights_arr(w: &TerrainWeights) -> [f32;4] {
    [w.grassland.max(0.0), w.forest.max(0.0), w.water.max(0.0), w.mountain.max(0.0)]
}

fn normalize(v: &mut [f32; 4]) {
    let s = v.iter().copied().sum::<f32>();
    if s > 0.0 {
        for x in v.iter_mut() {
            *x /= s;
        }
    } else {
        v[0] = 1.0;
        v[1] = 0.0;
        v[2] = 0.0;
        v[3] = 0.0;
    }
}

// Fill a blobby disk (slight ellipse jitter) of class `id` into `classes` and mark painted.
fn stamp_blob(
    classes: &mut Grid<u8>,
    painted: &mut [u8],
    allowed_mask: &[u8],
    area_mask: &[u8],
    center: IVec2,
    rmin: i32,
    rmax: i32,
    id: u8,
    rng: &mut Rng64,
) -> usize {
    let w = classes.w;
    let h = classes.h;
    let r = rng.range_i32(rmin, rmax).max(1);
    let r = clamp_radius_to_bounds(IVec2::new(w,h), center, r);
    // ellipse jitter to avoid perfect circles
    let ax = (1.0 - 0.15 + 0.30 * rng.f01()).max(0.7);
    let ay = (1.0 - 0.15 + 0.30 * rng.f01()).max(0.7);
    let mut placed = 0usize;
    let xmin = (center.x - r).max(0);
    let xmax = (center.x + r).min(w - 1);
    let ymin = (center.y - r).max(0);
    let ymax = (center.y + r).min(h - 1);
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            let dx = (x - center.x) as f32 / (r as f32 * ax);
            let dy = (y - center.y) as f32 / (r as f32 * ay);
            if dx*dx + dy*dy <= 1.0 {
                let i = idx(w, x, y);
                if allowed_mask[i] != 0 && area_mask[i] != 0 && painted[i] == 0 {
                    classes.set(x, y, id);
                    painted[i] = 1;
                    placed += 1;
                }
            }
        }
    }
    placed
}

// Build a circle mask as u8 array (1=inside)
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
            if dx*dx + dy*dy <= r2 {
                m[idx(w,x,y)] = 1;
            }
        }
    }
    m
}

/// New Phase 3: clumpy terrain that honors hard grass buffers around bases & shrines.
/// We process areas in ascending radius so small/strong rules occupy space first.
pub fn generate_phase3_terrain_clumps(
    tpl: &MapTemplate,
    base_centers: &[IVec2],
    shrines: &[IVec2],
    seed: u32,
) -> Grid<u8> {
    let size = IVec2::new(tpl.size.0, tpl.size.1);
    let w = size.x; let h = size.y;

    // Start as grass everywhere
    let mut classes = Grid::<u8>::new(w, h);
    for y in 0..h { for x in 0..w { classes.set(x,y, TERRAIN_GRASSLAND); } }

    // Hard "locked grass" mask: base disks + shrine disks
    let mut locked = vec![0u8; (w*h) as usize];
    // Base disks
    let br = tpl.player_spawns.base_radius.max(1);
    for &c in base_centers {
        let m = circle_mask(size, c, br);
        for i in 0..m.len() { if m[i]!=0 { locked[i]=1; } }
    }
    // Shrine disks
    let sr = tpl.terrain.shrine_grass_radius.max(0);
    for &s in shrines {
        let m = circle_mask(size, s, sr);
        for i in 0..m.len() { if m[i]!=0 { locked[i]=1; } }
    }
    // Apply locked grass now
    for y in 0..h { for x in 0..w {
        if locked[idx(w,x,y)] != 0 { classes.set(x,y, TERRAIN_GRASSLAND); }
    }}

    // Painted mask: tiles already assigned by an area (won't be repainted by larger areas)
    let mut painted = locked.clone();

    // Sort areas by priority: smaller radius first; tie-breaker Center before Spawn.
    let mut areas = tpl.terrain.areas.clone();
    areas.sort_by(|a,b| {
        use std::cmp::Ordering::*;
        match a.radius.cmp(&b.radius) {
            Equal => match (&a.source, &b.source) {
                (AreaSource::Center, AreaSource::Spawn) => Less,
                (AreaSource::Spawn,  AreaSource::Center) => Greater,
                _ => Equal,
            }
            other => other
        }
    });

    // Precompute some clump sizes
    let (f_min,f_max) = tpl.terrain.clumps.forest_patch;
    let (w_min,w_max) = tpl.terrain.clumps.water_patch;
    let (m_min,m_max) = tpl.terrain.clumps.mountain_patch;

    let mut rng = Rng64::new(seed as u64 ^ 0xA53C_9E37);

    for a in &areas {
        // Build area mask: union over spawns (if source=Spawn), or single circle at center
        let mut area_mask = vec![0u8; (w*h) as usize];
        match a.source {
            AreaSource::Center => {
                let c = IVec2::new(tpl.size.0/2, tpl.size.1/2);
                let m = circle_mask(size, c, a.radius);
                for i in 0..m.len() { if m[i]!=0 { area_mask[i]=1; } }
            }
            AreaSource::Spawn => {
                for &c in base_centers {
                    let m = circle_mask(size, c, a.radius);
                    for i in 0..m.len() { if m[i]!=0 { area_mask[i]=1; } }
                }
            }
        }

        // Available tiles to paint in this area now
        let mut avail: Vec<usize> = (0..(w*h) as usize)
            .filter(|&i| area_mask[i]!=0 && painted[i]==0)
            .collect();

        if avail.is_empty() { continue; }

        // Local target counts (approximate) for non-grass classes
        let mut mix = weights_arr(&a.weights);
        // emphasize this area's pull
        for k in 0..4 { mix[k] *= a.scale.max(0.0); }
        normalize(&mut mix);

        let target_total = avail.len() as f32;
        let mut target_forest   = (mix[TERRAIN_FOREST as usize]   * target_total).round() as i32;
        let mut target_water    = (mix[TERRAIN_WATER as usize]    * target_total).round() as i32;
        let mut target_mountain = (mix[TERRAIN_MOUNTAIN as usize] * target_total).round() as i32;

        // Helper: pick a random center from available
        let pick_center = |rng: &mut Rng64, avail: &Vec<usize>| -> Option<IVec2> {
            if avail.is_empty() { return None; }
            let i = avail[rng.choose_idx(avail.len())];
            let y = (i as i32) / w;
            let x = (i as i32) % w;
            Some(IVec2::new(x,y))
        };

        // Place clumps for a class until we hit the target or run out of attempts
        let mut place_class = |id: u8, (rmin,rmax): (i32,i32), target: &mut i32| {
            let mut tries = 0;
            while *target > 0 && tries < 4_000 {
                tries += 1;
                if let Some(c) = pick_center(&mut rng, &avail) {
                    let placed = stamp_blob(
                        &mut classes, &mut painted, &(!0u8).to_ne_bytes().repeat((w*h) as usize), // allowed=all (locked already in painted)
                        &area_mask, c, rmin, rmax, id, &mut rng
                    );
                    if placed == 0 { continue; }
                    *target -= placed as i32;
                    // Compact avail lazily every few blobs
                    if tries % 16 == 0 {
                        avail.retain(|&i| painted[i]==0 && area_mask[i]!=0);
                    }
                } else { break; }
            }
        };

        // Forest, then Water, then Mountain
        place_class(TERRAIN_FOREST,   (f_min,f_max), &mut target_forest);
        place_class(TERRAIN_WATER,    (w_min,w_max), &mut target_water);
        place_class(TERRAIN_MOUNTAIN, (m_min,m_max), &mut target_mountain);

        // Fill leftover (still unpainted in this area) with grass and mark as painted
        for i in avail {
            if painted[i]==0 && area_mask[i]!=0 {
                painted[i]=1;
                let x = (i as i32) % w; let y = (i as i32) / w;
                classes.set(x,y, TERRAIN_GRASSLAND);
            }
        }
    }

    // Ensure locked grass stays grass (defensive)
    for y in 0..h { for x in 0..w {
        if locked[idx(w,x,y)]!=0 { classes.set(x,y, TERRAIN_GRASSLAND); }
    }}

    classes
}
