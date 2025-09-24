mod terrain;

use terrain::generate::{generate_all_phases};

fn main() {
    let tpl = terrain::template::MapTemplate::from_file("assets/maps/haunted_woods.ron");
    let num_bases = 6usize;
    let start_angle_deg = 0.0;

    // choose any seed to stabilize the texture of terrain
    let terrain_seed = 123456;

    let (_p1, _ley, _final) = generate_all_phases(
        &tpl,
        num_bases,    // num_bases
        start_angle_deg, // start_angle_deg
        None,         // ley: use template (or defaults)
        None,         // blend: use template (or defaults)
        None,         // fractal: use template (or defaults)
        terrain_seed, // terrain_seed
        Some("out"),  // PNG dir or None
    );
}
