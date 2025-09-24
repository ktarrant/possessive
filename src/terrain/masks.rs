use bitvec::prelude::*;
use glam::IVec2;

#[derive(Clone)]
pub struct Mask {
    pub w: i32,
    pub h: i32,
    bits: BitVec,
}
impl Mask {
    pub fn new(w: i32, h: i32) -> Self { Self { w, h, bits: bitvec![0; (w*h) as usize] } }
    #[inline] fn idx(&self, x: i32, y: i32) -> usize { (y*self.w + x) as usize }

    pub fn set(&mut self, x: i32, y: i32, v: bool) {
        let i = self.idx(x, y);
        self.bits.set(i, v);
    }

    pub fn get(&self, x: i32, y: i32) -> bool { self.bits[self.idx(x,y)] }

    // out = self & !other
    pub fn and_not(&self, other: &Mask) -> Mask {
        let mut out = self.clone();
        for i in 0..out.bits.len() {
            if other.bits[i] { out.bits.set(i, false); }
        }
        out
    }

    // out = self & other
    pub fn and(&self, other: &Mask) -> Mask {
        let mut out = self.clone();
        // BitAndAssign<&BitSlice> is implemented
        out.bits &= other.bits.as_bitslice();
        out
    }

    pub fn iter_true(&self) -> impl Iterator<Item=IVec2> + '_ {
        self.bits.iter_ones().map(move |i| IVec2::new((i as i32)%self.w, (i as i32)/self.w))
    }
}
