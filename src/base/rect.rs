use crate::base::Pos;

/// 矩形領域（含む-含む）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub min: Pos,
    pub max: Pos,
}

impl Rect {
    pub fn new(min: Pos, max: Pos) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.y >= self.min.y
            && pos.y <= self.max.y
    }
}
