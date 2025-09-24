use rand::{SeedableRng, Rng};
use rand_pcg::Pcg64Mcg;

#[derive(Clone)]
pub struct RngSeq { base: u64 }
impl RngSeq {
    pub fn new(seed: u64) -> Self { Self { base: seed } }
    pub fn for_phase(&self, phase: u64) -> Pcg64Mcg {
        Pcg64Mcg::seed_from_u64(self.base ^ (phase.wrapping_mul(0x9E3779B97F4A7C15)))
    }
}
pub fn rand_unit(rng: &mut Pcg64Mcg) -> f32 { rng.gen::<f32>() }
