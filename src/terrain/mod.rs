// src/terrain/mod.rs
pub mod template;
pub mod grid;
pub mod rng;
pub mod noise;
pub mod masks;
pub mod poisson;
pub mod river;
pub mod placement;
pub mod generate;
pub mod debug_png;

// (optional) re-exports so main.rs can use short paths
pub use generate::{generate_map, GeneratedMap};
pub use template::{MapTemplate, Symmetry};
