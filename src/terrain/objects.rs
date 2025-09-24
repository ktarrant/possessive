use glam::IVec2;

use super::grid::Grid;
use super::template::{MapTemplate, Region};
use super::landscape::{
    TERRAIN_GRASSLAND, TERRAIN_FOREST, TERRAIN_WATER, TERRAIN_MOUNTAIN,
};

// ---------------- tiny deterministic RNG ----------------
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
    fn range_usize(&mut self, hi_excl: usize) -> usize {
        if hi_excl == 0 { 0 } else { (self.next_u32() as usize) % hi_excl }
    }
}

#[inline] fn idx(w: i32, x: i32, y: i32) -> usize { (y * w + x) as usize }

// Paint an EXCLUSION disk (set to 0) into the mask
fn paint_exclusion_disk(mask: &mut [u8], w: i32, h: i32, c: IVec2, r: i32) {
    if r <= 0 { return; }
    let r2 = r * r;
    let xmin = (c.x - r).max(0);
    let xmax = (c.x + r).min(w - 1);
    let ymin = (c.y - r).max(0);
    let ymax = (c.y + r).min(h - 1);
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            let dx = x - c.x; let dy = y - c.y;
            if dx*dx + dy*dy <= r2 {
                mask[idx(w,x,y)] = 0;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlacedObject {
    pub pos: IVec2,
    /// index into tpl.objects.types
    pub kind: u16,
}

/// Multi-type placement with:
/// - Per-region densities
/// - Global cross-type spacing: min distance = max(radius_a, radius_b)
/// - Exclusion rings around bases/shrines (no placement inside)
pub fn generate_objects(
    tpl: &MapTemplate,
    classes: &Grid<u8>,          // from Phase 3/4
    base_centers: &[IVec2],
    shrines: &[IVec2],
    extra_seed: u32,
) -> Vec<PlacedObject> {
    let w = classes.w;
    let h = classes.h;
    let total = (w * h) as usize;

    // --------------- exclusion mask (1 = allowed, 0 = forbidden) ---------------
    let mut allowed: Vec<u8> = vec![1; total];

    // Exclude base build rings (use base_radius)
    let base_r = tpl.player_spawns.base_radius.max(0);
    for &c in base_centers {
        paint_exclusion_disk(&mut allowed, w, h, c, base_r);
    }

    // Exclude shrine rings (use shrine_grass_radius from terrain rules, if present)
    // If your TerrainRules doesn't have this field, set sr = 0 or whichever field you use.
    let sr = tpl.terrain.shrine_grass_radius.max(0);
    for &s in shrines {
        paint_exclusion_disk(&mut allowed, w, h, s, sr);
    }

    // --------------- precompute region tile lists (only ALLOWED tiles) ----------
    let mut tiles_grass = Vec::<usize>::new();
    let mut tiles_forest = Vec::<usize>::new();
    let mut tiles_water = Vec::<usize>::new();
    let mut tiles_mountain = Vec::<usize>::new();

    for y in 0..h {
        for x in 0..w {
            let i = idx(w,x,y);
            if allowed[i] == 0 { continue; }
            match *classes.get(x, y) {
                TERRAIN_GRASSLAND => tiles_grass.push(i),
                TERRAIN_FOREST    => tiles_forest.push(i),
                TERRAIN_WATER     => tiles_water.push(i),
                TERRAIN_MOUNTAIN  => tiles_mountain.push(i),
                _ => {}
            }
        }
    }

    let region_tiles = |r: Region| -> &Vec<usize> {
        match r {
            Region::Grassland => &tiles_grass,
            Region::Forest    => &tiles_forest,
            Region::Water     => &tiles_water,
            Region::Mountain  => &tiles_mountain,
        }
    };

    // --------------- spatial buckets for O(1) neighbor checks -------------------
    let mut max_r = 1i32;
    for t in &tpl.objects.types {
        if t.radius > max_r { max_r = t.radius; }
    }
    let cell = max_r.max(1);
    let bw = ((w + cell - 1) / cell) as i32;
    let bh = ((h + cell - 1) / cell) as i32;
    let nbuckets = (bw * bh) as usize;
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); nbuckets];

    struct PO { pos: IVec2, kind: u16, r: i32 }
    let mut placed: Vec<PO> = Vec::new();

    let mut rng = Rng64::new(((tpl.objects.base_seed as u64) << 32) ^ (extra_seed as u64));

    let bxy = |p: IVec2| -> (i32, i32) { (p.x / cell, p.y / cell) };
    let bidx = |bx: i32, by: i32| -> usize { (by * bw + bx) as usize };

    let can_place = |p: IVec2, r_new: i32,
                     placed: &Vec<PO>, buckets: &Vec<Vec<usize>>| -> bool {
        let (bx, by) = bxy(p);
        let bx0 = (bx - 1).max(0);
        let by0 = (by - 1).max(0);
        let bx1 = (bx + 1).min(bw - 1);
        let by1 = (by + 1).min(bh - 1);
        for yy in by0..=by1 {
            for xx in bx0..=bx1 {
                let bi = bidx(xx, yy);
                for &pi in &buckets[bi] {
                    let q = placed[pi].pos;
                    let r_other = placed[pi].r;
                    let dmin = r_new.max(r_other);
                    let dx = p.x - q.x;
                    let dy = p.y - q.y;
                    if dx*dx + dy*dy < dmin * dmin {
                        return false;
                    }
                }
            }
        }
        true
    };

    // --------------- place by descending radius ---------------------------------
    let mut type_order: Vec<usize> = (0..tpl.objects.types.len()).collect();
    type_order.sort_by_key(|&i| -(tpl.objects.types[i].radius));

    for ti in type_order {
        let tr = &tpl.objects.types[ti];
        if tr.per_region.is_empty() { continue; }

        for rr in &tr.per_region {
            let tiles = region_tiles(rr.region);
            if tiles.is_empty() { continue; }

            let area_tiles = tiles.len() as f32; // allowed tiles only
            let per_unit2 = if rr.density.area > 0.0 { rr.density.per_unit2() } else { 0.0 };
            if per_unit2 <= 0.0 { continue; }

            let mut target = (per_unit2 * area_tiles).round() as i32;
            if target <= 0 { continue; }

            let mut attempts = 0i32;
            let max_attempts = target * 50; // tune as needed

            while target > 0 && attempts < max_attempts {
                attempts += 1;
                let pick = tiles[rng.range_usize(tiles.len())];
                let y = (pick as i32) / w;
                let x = (pick as i32) % w;
                let p = IVec2::new(x, y);

                // Still double-check allowed (cheap)
                if allowed[pick] == 0 { continue; }

                if can_place(p, tr.radius, &placed, &buckets) {
                    let (bx, by) = bxy(p);
                    let bi = bidx(bx, by);
                    let idx_new = placed.len();
                    placed.push(PO { pos: p, kind: ti as u16, r: tr.radius });
                    buckets[bi].push(idx_new);
                    target -= 1;
                }
            }
        }
    }

    placed.into_iter().map(|po| PlacedObject { pos: po.pos, kind: po.kind }).collect()
}
