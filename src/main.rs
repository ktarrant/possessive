mod terrain;

use terrain::{MapTemplate};
use terrain::generate::{LeyConfig, generate_step_by_step_and_save};

fn main() {
    let tpl = MapTemplate::from_file("assets/maps/phase1_example.ron");
    let num_bases = 4usize;
    let start_angle_deg = 0.0;

    let ley_cfg = LeyConfig {
        m_shrines: num_bases,                          // same count as bases
        shrine_ring: tpl.player_spawns.center_radius,  // same ring; try +/- to move in/out
        offset_deg: 180.0 / (num_bases as f32),        // half-step between bases
        connect_cycle: true,
        connect_spokes: true,
    };

    let (_p1, _ley) = generate_step_by_step_and_save(
        &tpl, num_bases, start_angle_deg, ley_cfg, "out"
    );
}
