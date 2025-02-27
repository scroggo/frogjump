use crate::math;
use godot::prelude::*;

// Global positions for two end points of a surface, along with the normal.
// Note that in some cases, the end point is just the end of a tile, and the
// surface may extend further.
#[derive(Debug, Clone, Copy)]
pub struct LandingSurface {
    // By convention, if the player landed on a corner, it should be `a`.
    pub a: Vector2,
    pub b: Vector2,
    pub normal: Vector2,
}

impl LandingSurface {
    pub fn new(a: Vector2, b: Vector2, player_motion: Vector2) -> Option<LandingSurface> {
        if let Some(normal) = math::normal(a, b, player_motion) {
            return Some(LandingSurface { a, b, normal });
        }
        None
    }

    pub fn length_squared(&self) -> f32 {
        (self.a - self.b).length_squared()
    }
}
