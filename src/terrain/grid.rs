use glam::IVec2;

#[derive(Clone)]
pub struct Grid<T> {
    pub w: i32,
    pub h: i32,
    data: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h, data: vec![T::default(); (w*h) as usize] }
    }
}

impl<T> Grid<T> {
    #[inline] fn idx(&self, x: i32, y: i32) -> usize { (y*self.w + x) as usize }
    pub fn in_bounds(&self, p: IVec2) -> bool { p.x>=0 && p.y>=0 && p.x<self.w && p.y<self.h }
    pub fn get(&self, x: i32, y: i32) -> &T { &self.data[self.idx(x,y)] }
    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut T { let i=self.idx(x,y); &mut self.data[i] }
    pub fn set(&mut self, x: i32, y: i32, v: T) {
        let i = self.idx(x,y);
        self.data[i] = v;
    }
    pub fn iter_xy(&self) -> impl Iterator<Item=IVec2> + '_ {
        (0..self.h).flat_map(move |y| (0..self.w).map(move |x| IVec2::new(x,y)))
    }
}

pub fn clamp(p: IVec2, w: i32, h: i32) -> IVec2 {
    IVec2::new(p.x.clamp(0, w-1), p.y.clamp(0, h-1))
}
