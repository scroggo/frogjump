use godot::prelude::*;

// Assuming the player collided with the surface specified by points (a, b),
// Return the normal vector pointing towards the player.
pub fn normal(a: Vector2, b: Vector2, player_motion: Vector2) -> Vector2 {
    let ortho = (b - a).orthogonal();
    (-player_motion).project(ortho).normalized()
}