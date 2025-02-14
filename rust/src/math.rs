use godot::global::absf;
use godot::prelude::*;

// Assuming the player collided with the surface specified by points (a, b),
// Return the normal vector pointing towards the player.
pub fn normal(a: Vector2, b: Vector2, player_motion: Vector2) -> Option<Vector2> {
    let ortho = (b - a).orthogonal();
    (-player_motion).project(ortho).try_normalized()
}

// Succinct version. (Using Godot's `pow` requires conversion to/from f64.)
// TODO: Convert to an extension trait?
pub fn squared(n: f32) -> f32 {
    n * n
}

pub fn same_normals_approx(n1: Vector2, n2: Vector2) -> bool {
    const TOLERANCE: f64 = 0.05;
    absf(n1.angle_to(n2) as f64) < TOLERANCE
}
