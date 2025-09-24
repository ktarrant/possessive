mod terrain;

use terrain::ley::{LeyConfig};
use terrain::generate::{generate_all_phases_and_save};

fn main() {
    let tpl = terrain::template::MapTemplate::from_file("assets/maps/phase1_example.ron");
    let num_bases = 6usize;
    let start_angle_deg = 0.0;

    let ley_cfg = LeyConfig {
        m_shrines: num_bases,
        shrine_ring: tpl.player_spawns.center_radius,
        offset_deg: 180.0 / (num_bases as f32),
        connect_cycle: true,
        connect_spokes: true,
    };

    // choose any seed to stabilize the texture of terrain
    let terrain_seed = 123456;

    let (_p1, _ley, _classes) = generate_all_phases_and_save(
        &tpl,
        num_bases,
        start_angle_deg,
        ley_cfg,
        terrain_seed,
        "out",
    );
}
