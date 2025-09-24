use glam::IVec2;

#[derive(Clone)]
pub struct Grid<T> {
    pub w: i32,
    pub h: i32,
    data: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h, data: vec![T::default(); (w * h) as usize] }
    }
}

impl<T> Grid<T> {
    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize { (y * self.w + x) as usize }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, v: T) {
        let i = self.idx(x, y);
        self.data[i] = v;
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> &T {
        &self.data[self.idx(x, y)]
    }

    #[allow(dead_code)]
    pub fn in_bounds(&self, p: IVec2) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h
    }
}
