use crate::math;
use godot::classes::Geometry2D;
use godot::global::absf;
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

    // It is possible to create a `LandingSurface` with the normal pointing in
    // the wrong direction. Given the polygon of which it is a part, correct the
    // normal so that it points outside the polygon.
    // TODO: Update existing call sites to compute the normal in a similar
    // manner?
    pub fn correct_normal(self, polygon: &PackedVector2Array) -> LandingSurface {
        // Pick a test point just off the middle of the line segment. Assuming
        // relatively simple shapes, this should determine the proper direction.
        let test_point = (self.a + self.b) / 2.0 + self.normal;

        // Warning: there are known bugs in this method, e.g.
        // https://github.com/godotengine/godot/issues/82305. So far it seems to
        // work in my test cases.
        if Geometry2D::singleton().is_point_in_polygon(test_point, polygon) {
            godot_print!("Correcting normal!");

            // Check the other direction, too. If that point is *also* in the
            // polygon, we'll need to revisit this.
            let test_point2 = test_point + -2.0 * self.normal;
            if Geometry2D::singleton().is_point_in_polygon(test_point2, polygon) {
                godot_error!("Both test points are in polygon!");
                godot_print!(
                    "Both test points, {test_point} and {test_point2}, are in {polygon:?}"
                );

                // Both are no good, so use the original?
                return self;
            }
            return LandingSurface {
                a: self.a,
                b: self.b,
                normal: -self.normal,
            };
        }
        return self;
    }

    // Given two indices on a polygon, which are either consecutive or can be
    // treated as such, return a `LandingSurface` (if any).
    pub fn find_surface(
        polygon: &PackedVector2Array,
        i: usize,
        i2: usize,
    ) -> Option<LandingSurface> {
        let a = polygon[i];
        let b = polygon[i2];
        let normal = (a - b).orthogonal().try_normalized()?;
        let surface = LandingSurface { a, b, normal };
        Some(surface.correct_normal(polygon))
    }

    pub fn hit_by(&self, collision_position: Vector2, collision_normal: Vector2) -> bool {
        // A plane is defined by a normal and a distance from the origin.
        let d = self.normal.dot(self.a);

        // Compute distance from collision to the plane.
        let distance = self.normal.dot(collision_position) - d;
        const TOLERANCE: f64 = 0.2;
        if absf(distance as f64) < TOLERANCE {
            godot_print!(
                "Collision {collision_position} is {distance} from side {}-{}",
                self.a,
                self.b
            );
            // Make sure the normals match.
            let dot_product = self.normal.dot(collision_normal);
            if ((1.0 - dot_product) as f64) < TOLERANCE {
                // If the collision is between the two points, the length of ab
                // should roughly equal ac + bc.
                let ab = (self.a - self.b).length();
                let ac = (self.a - collision_position).length();
                let bc = (self.b - collision_position).length();
                let diff = ab - (ac + bc);
                return absf(diff as f64) < TOLERANCE;
            }
        }
        false
    }
}
