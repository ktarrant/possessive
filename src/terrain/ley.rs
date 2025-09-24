use glam::{IVec2, Vec2};

/// Simple configuration for Phase 2 ley network.
pub struct LeySettings {
    pub m_shrines: usize,   // how many shrines
    pub shrine_ring: i32,   // ring radius for shrines
    pub offset_deg: f32,    // angular offset vs. base ring start (e.g., 180/n for half-step)
    pub connect_cycle: bool,
    pub connect_spokes: bool,
}

/// Raw ley output for Phase 2
pub struct LeyNetwork {
    pub shrines: Vec<IVec2>,
    /// Line segments as pairs of points (pixel/ tile space)
    pub lines: Vec<(IVec2, IVec2)>,
}

/// Place `m_shrines` evenly on a ring of radius `shrine_ring`,
/// offset by `offset_deg` relative to Phase 1 base ring start angle.
/// Optionally connect shrines in a cycle and/or draw center spokes.
pub fn generate_ley(
    map_size: (i32, i32),
    num_bases: usize,
    start_angle_deg: f32,
    m_shrines: usize,
    shrine_ring: i32,
    offset_deg: f32,
    connect_cycle: bool,
    connect_spokes: bool,
) -> LeyNetwork {
    assert!(num_bases >= 1 && m_shrines >= 1);
    let size = glam::ivec2(map_size.0, map_size.1);
    let center = size.as_vec2() / 2.0;

    let start_angle = start_angle_deg.to_radians();
    let offset = offset_deg.to_radians();

    // Shrine positions
    let mut shrines = Vec::with_capacity(m_shrines);
    for j in 0..m_shrines {
        let ang = start_angle + offset + (j as f32) * std::f32::consts::TAU / (m_shrines as f32);
        let p = center + Vec2::new(ang.cos(), ang.sin()) * (shrine_ring as f32);
        shrines.push(IVec2::new(p.x.round() as i32, p.y.round() as i32));
    }

    // Build ley lines
    let mut lines = Vec::new();

    if connect_cycle {
        for j in 0..m_shrines {
            let a = shrines[j];
            let b = shrines[(j + 1) % m_shrines];
            lines.push((a, b));
        }
    }

    if connect_spokes {
        let c = IVec2::new(center.x.round() as i32, center.y.round() as i32);
        for &s in &shrines {
            lines.push((c, s));
        }
    }

    LeyNetwork { shrines, lines }
}
