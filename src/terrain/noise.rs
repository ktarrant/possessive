use noise::{NoiseFn, Fbm, RidgedMulti};
use noise::MultiFractal; // needed for set_octaves

pub enum LayerKind { Fbm, Ridged }
pub struct Layer { pub freq: f64, pub amp: f32, pub octaves: usize, pub kind: LayerKind }

pub fn layer_value(seed: u32, x: f64, y: f64, layer: &Layer) -> f32 {
    match layer.kind {
        LayerKind::Fbm => {
            let fbm = Fbm::<noise::Perlin>::new(seed).set_octaves(layer.octaves);
            (fbm.get([x * layer.freq, y * layer.freq]) as f32) * layer.amp
        }
        LayerKind::Ridged => {
            let rid = RidgedMulti::<noise::Perlin>::new(seed).set_octaves(layer.octaves);
            (rid.get([x * layer.freq, y * layer.freq]) as f32) * layer.amp
        }
    }
}
