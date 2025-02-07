use std::f32::consts::PI;
use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::jump_handler::{JumpDetector, JumpHandler};
use crate::math;
use godot::classes::{
    AnimatedSprite2D, CharacterBody2D, CollisionShape2D, Geometry2D, ICharacterBody2D, InputEvent,
    InputEventScreenTouch, KinematicCollision2D, TileMapLayer, Timer,
};
use godot::global::{absf, cos, pow, randf_range};
use godot::prelude::*;

struct TouchJumpHandler {
    touch_is_pressed: Arc<AtomicBool>,
}

impl TouchJumpHandler {
    fn new(atomic_bool: &Arc<AtomicBool>) -> Self {
        TouchJumpHandler {
            touch_is_pressed: atomic_bool.clone(),
        }
    }
}

impl JumpDetector for TouchJumpHandler {
    fn is_jump_pressed(&self) -> bool {
        self.touch_is_pressed
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

// TODO: Move to its own module?
#[derive(PartialEq, GodotConvert, Var, Export, Clone, Copy)]
#[godot(via=GString)]
pub enum Direction {
    Left,
    Right,
}

impl Default for Direction {
    fn default() -> Self {
        // Platformers traditionally play to the right.
        Self::Right
    }
}

impl Not for Direction {
    fn not(self) -> Self::Output {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    type Output = Self;
}

// Useful info for spawning a new Player.
#[derive(Default, Clone, Copy)]
pub struct PlayerInfo {
    // Position local to the parent.
    pos: Vector2,
    vel: Vector2,
    dir: Direction,
}

// Global positions for two end points of a surface, along with the normal.
// Note that in some cases, the end point is just the end of a tile, and the
// surface may extend further.
#[derive(Debug, Clone, Copy)]
struct LandingSurface {
    a: Vector2,
    b: Vector2,
    normal: Vector2,
}

impl LandingSurface {
    fn new(a: Vector2, b: Vector2, player_motion: Vector2) -> LandingSurface {
        LandingSurface {
            a,
            b,
            normal: math::normal(a, b, player_motion),
        }
    }
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    #[export]
    direction: Direction,
    #[export]
    target_velocity: Vector2,
    #[export]
    max_jump_strength: f32,
    #[export]
    fall_acceleration: f32,
    #[export]
    on_surface: bool, // True when on any surface: floor, wall, ceiling.
    on_ceiling: bool,
    #[export]
    shimmy_speed: f32,
    // If the player lands on a corner, they will "shimmy" until they're fully on
    // the surface
    shimmy_dest: Option<Vector2>,
    // Whether the screen is being touched, which is an alternative to using the
    // "jump" input action. Uses reference counting since it will be shared with
    // the `TouchJumpHandler`. (Note: Maybe an `RwLock` would be more clear,
    // since the handler will only read it.) Uses atomics out of caution. (I do
    // not think Godot will access from multiple threads, but I'm still
    // learning Godot.)
    touch_is_pressed: Arc<AtomicBool>,
    // Keep track of whether the player has jumped in order to decide whether to
    // switch to touches. Only switch prior to any jumps.
    has_jumped: bool,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            direction: Direction::Left,
            target_velocity: Vector2::ZERO,
            max_jump_strength: 20.0,
            fall_acceleration: 75.0,
            on_surface: false,
            on_ceiling: false,
            shimmy_speed: 75.0,
            shimmy_dest: None,
            touch_is_pressed: Arc::new(AtomicBool::new(false)),
            has_jumped: false,
            base,
        }
    }

    fn ready(&mut self) {
        let blink = self.base().callable("on_blink_timeout");
        let mut blink_timer = self.blink_timer();
        blink_timer.connect("timeout", &blink);
        self.start_blink_timer();

        if self.direction == Direction::Right {
            self.sprite().set_flip_h(true);
        }
        if self.on_surface {
            self.sprite().set_frame(0);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let old_position = self.base().get_position();
        if self.shimmy_dest.is_some() {
            let shimmy_dest = self.shimmy_dest.unwrap();
            let direction = (shimmy_dest - old_position).normalized();
            let movement = direction * self.shimmy_speed * delta as f32;
            let mut new_position = old_position + movement;
            let distance_squared_from_dest = new_position.distance_squared_to(shimmy_dest);
            if distance_squared_from_dest < 1.0 {
                // Avoid overshooting the destination.
                new_position = shimmy_dest;
                self.shimmy_dest = None;
            }
            self.base_mut().set_position(new_position);
            if self.shimmy_dest.is_none() {
                self.sprite().play_ex().name("default").done();
            }
            return;
        }
        if !self.on_surface {
            self.target_velocity.y += delta as f32 * self.fall_acceleration;
        }
        let motion = self.target_velocity * delta as f32;
        let collision_opt = self.base_mut().move_and_collide(motion);
        if let Some(collision) = collision_opt {
            let new_position = self.base().get_position();
            godot_print!(
                "moved from {old_position} to {new_position}; diff: {}",
                new_position - old_position
            );
            print_collision(&collision);
            if let Some(collider) = collision.get_collider() {
                godot_print!("Collided with {:?}", collider);

                // TODO: The check for `is_zero_approx` avoids a divide by zero, but is only
                // necessary because the manual rotation doesn't (yet) ensure that it doesn't
                // create a new collision.
                if collision.get_depth() > 0.0 && !motion.is_zero_approx() {
                    // The player is penetrating the wall. Move back along the
                    // direction of motion far enough to remove the overlap.
                    let reverse_motion = -motion.normalized();
                    let depth_vector = collision.get_depth() * collision.get_normal();
                    let offset = depth_vector.length()
                        / cos(reverse_motion.angle_to(depth_vector).into()) as f32;
                    let position = self.base().get_position();
                    self.base_mut()
                        .set_position(position + offset * reverse_motion);
                    godot_print!("Moving back by {offset} along {reverse_motion} for change of {} from {position} to {}",
                        offset * reverse_motion, self.base().get_position());
                }
                let mut landing_surface: Option<LandingSurface> = None;
                let collision_position = collision.get_position();
                if let Some(points) = get_collider_points(collider, &collision_position) {
                    godot_print!("Returned points: {points}");
                    if let Some(index) = points.find(collision_position, None) {
                        godot_print!("hit a corner!");

                        landing_surface = self.pick_side_to_land_on_from_corner(
                            &points,
                            index,
                            motion,
                            collision.get_normal(),
                        );
                    } else {
                        landing_surface = self.pick_side_to_land_on(
                            &points,
                            collision_position,
                            motion,
                            collision.get_normal(),
                        );
                    }
                }
                godot_print!("Landing surface: {landing_surface:?}");
                self.on_surface = true;

                // Reverse the jump animation to land.
                self.sprite()
                    .play_ex()
                    .name("jump")
                    .custom_speed(-3.0)
                    .from_end(true)
                    .done();

                let normal = landing_surface
                    .map_or_else(|| collision.get_normal(), |surface| surface.normal);
                let new_angle = normal.angle() + PI / 2.0;
                godot_print!("rotating to angle {}", new_angle);
                self.base_mut().set_rotation(new_angle);
                match normal {
                    Vector2 { x, y: _ } if x > 0.5 => {
                        self.direction = Direction::Right;
                        self.sprite().set_flip_h(false);
                    }
                    Vector2 { x, y: _ } if x < -0.5 => {
                        self.direction = Direction::Left;
                        self.sprite().set_flip_h(true);
                    }
                    Vector2 { x: _, y } if y > 0.5 => {
                        self.on_ceiling = true;
                        let flip_h = self.direction == Direction::Left;
                        self.sprite().set_flip_h(flip_h);
                    }
                    Vector2 { x: _, y } if y < -0.5 => {
                        let flip_h = self.direction == Direction::Right;
                        self.sprite().set_flip_h(flip_h);
                    }
                    normal => {
                        godot_error!("Landed with surprise normal {normal}");
                    }
                } // match

                // Now that we've rotated the player in the proper direction,
                // move them so they are properly on their new surface.
                if landing_surface.is_some() {
                    // When landing on a corner, `a` represents the corner.
                    // TODO: This is totally arbitrary. Enforce/make clearer.
                    if collision_position == landing_surface.unwrap().a {
                        // Land on the corner directly, pushed away by the
                        // normal. If this is too jarring in some cases, we can
                        // try starting from the player's position.
                        let global_position =
                            landing_surface.unwrap().a + normal * (self.height() / 2.0);
                        let new_player_position = self
                            .base()
                            .get_parent()
                            .unwrap()
                            .cast::<Node2D>()
                            .to_local(global_position);

                        // Shimmy more fully onto the surface over the next
                        // several frames.
                        let surface_direction =
                            (landing_surface.unwrap().b - landing_surface.unwrap().a).normalized();

                        // The slop means the players body may not be entirely
                        // on the surface, but at least the feet should be.
                        const SLOP: f32 = 0.7;
                        self.shimmy_dest = Some(
                            new_player_position + (self.width() / 2.0) * surface_direction * SLOP,
                        );
                        self.sprite().play_ex().name("shimmy").done();
                        self.base_mut().set_position(new_player_position);
                    } else {
                        // Line up the player so that they appear to be resting directly
                        // on the surface.

                        // Need the global position for the player, since the surface
                        // uses global positions.
                        let global_position = self
                            .base()
                            .get_parent()
                            .unwrap()
                            .cast::<Node2D>()
                            .to_global(self.base().get_position());

                        // We have the normal for the plane, we just need its
                        // distance from the origin.
                        let d = normal.dot(landing_surface.unwrap().a);

                        let distance_to_surface = normal.dot(global_position) - d;
                        let desired_distance = self.height() / 2.0;
                        let dist_to_move = desired_distance - distance_to_surface;
                        let new_player_position =
                            self.base().get_position() + dist_to_move * normal;
                        self.base_mut().set_position(new_player_position);
                    }
                    godot_print!("Player's local position: {}", self.base().get_position());
                    self.check_collisions();
                }
            }
        }

        if self.on_surface {
            if let Some(jump_strength) = self.jump_handler().bind_mut().handle_input(delta) {
                if !self.on_ceiling {
                    // Jump right-side up, facing `Direction`.
                    self.base_mut().set_rotation(0.0);
                    let flip_h = self.direction == Direction::Right;
                    self.sprite().set_flip_h(flip_h);
                }
                self.target_velocity = self.get_jump(jump_strength);
                self.sprite().play_ex().name("jump").done();
                self.on_surface = false;
                self.on_ceiling = false;
                self.has_jumped = true;
            } else {
                self.target_velocity = Vector2::ZERO;
            }
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Some(touch_event) = event.try_cast::<InputEventScreenTouch>().ok() {
            if touch_event.is_pressed() && !self.has_jumped {
                self.use_touch();
            }
            self.touch_is_pressed.store(
                touch_event.is_pressed(),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
    }
}

#[godot_api]
impl Player {
    fn get_jump(&self, jump_ratio: f32) -> Vector2 {
        let ceiling_multiplier = match self.on_ceiling {
            true => 1.0,
            false => -1.0,
        };
        let jump_strength = jump_ratio * self.max_jump_strength * ceiling_multiplier;
        const JUMP_ANGLE: f32 = 5.0 * PI / 16.0;
        let jump_angle = match self.direction {
            Direction::Left => JUMP_ANGLE,
            Direction::Right => -JUMP_ANGLE,
        } * ceiling_multiplier;
        Vector2::new(0.0, jump_strength).rotated(jump_angle)
    }

    fn blink_timer(&self) -> Gd<Timer> {
        self.base().get_node_as::<Timer>("BlinkTimer")
    }

    fn start_blink_timer(&self) {
        let wait_time = randf_range(3.0, 7.0);
        self.blink_timer().start_ex().time_sec(wait_time).done();
    }

    #[func]
    fn on_blink_timeout(&self) {
        let mut sprite = self.sprite();
        if self.on_surface && !sprite.is_playing() {
            sprite.play_ex().name("blink").done();
        }
        self.start_blink_timer();
    }

    fn sprite(&self) -> Gd<AnimatedSprite2D> {
        self.base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D")
    }

    fn jump_handler(&self) -> Gd<JumpHandler> {
        self.base().get_node_as::<JumpHandler>("JumpHandler")
    }

    fn use_touch(&mut self) {
        let detector = Box::new(TouchJumpHandler::new(&self.touch_is_pressed));
        self.jump_handler()
            .bind_mut()
            .replace_jump_detector(detector);
    }

    pub fn get_player_info(&self) -> PlayerInfo {
        PlayerInfo {
            pos: self.base().get_position(),
            vel: self.target_velocity,
            dir: self.direction,
        }
    }

    pub fn set_player_info(&mut self, info: &PlayerInfo) {
        self.base_mut().set_position(info.pos);
        self.target_velocity = info.vel;
        self.direction = info.dir;
    }

    fn pick_side_to_land_on(
        &self,
        points: &PackedVector2Array,
        collision_position: Vector2,
        player_motion: Vector2,
        collision_normal: Vector2,
    ) -> Option<LandingSurface> {
        for i in 0..points.len() {
            let i2 = next_point(points, i);
            let a = points[i];
            let b = points[i2];

            // Construct a plane from a normal and a point. See
            // https://docs.godotengine.org/en/stable/tutorials/math/vectors_advanced.html#constructing-a-plane-in-2d
            let n = math::normal(a, b, player_motion);
            let d = n.dot(a);

            // If the collision is in this plane and the normal matches, this is
            // the side we landed on.
            let distance = n.dot(collision_position) - d;
            // TODO: Fine-tune this tolerance. The first collision I measured
            // had a `distance` of 0, so maybe it's not necessary at all, but
            // I would expect that there might be rounding errors.
            const TOLERANCE: f64 = 0.05;
            if absf(distance as f64) < TOLERANCE {
                godot_print!("Collision {collision_position} is {distance} from side {a}-{b}");
                let dot_product = n.dot(collision_normal);
                if ((1.0 - dot_product) as f64) < TOLERANCE {
                    // TODO: Make sure the side is long enough.
                    // TODO: Check if it's close to the corner?
                    return Some(LandingSurface { a, b, normal: n });
                }
            }
        }
        None
    }

    fn pick_side_to_land_on_from_corner(
        &self,
        points: &PackedVector2Array,
        index: usize,
        player_motion: Vector2,
        collision_normal: Vector2,
    ) -> Option<LandingSurface> {
        if player_motion.is_zero_approx() {
            // I hope to be able to avoid this by properly positioning the player such that new
            // collisions are not generated.
            godot_error!("No player motion!");
            return None;
        }

        let mut landing_surface_a =
            Self::pick_adjacent_side(points, index, prior_point, player_motion);
        let mut landing_surface_b =
            Self::pick_adjacent_side(points, index, next_point, player_motion);
        // If these two normals are close enough, treat them as a continuous
        // surface.
        const TOLERANCE: f64 = 0.05;
        if absf(landing_surface_a.normal.angle_to(landing_surface_b.normal) as f64) < TOLERANCE {
            godot_print!("Treat two sides as same surface!");
            let landing_surface_full =
                LandingSurface::new(landing_surface_a.b, landing_surface_b.b, player_motion);
            if !self.can_land_on_surface(&landing_surface_full) {
                godot_error!("Surfaces are too small! Landing anyway!");
            }
            return Some(landing_surface_full);
        }
        // First pick the surface whose normal is closest to the collision normal.
        // Since we're dealing with normals, we can just use the one with the dot
        // product that is larger.
        if landing_surface_b.normal.dot(collision_normal)
            > landing_surface_a.normal.dot(collision_normal)
        {
            std::mem::swap(&mut landing_surface_a, &mut landing_surface_b);
        }

        for surface in [&landing_surface_a, &landing_surface_b] {
            if self.can_land_on_surface(surface) {
                return Some(*surface);
            }
        }
        // TODO: We might hit this case if you land close to the branch - then we'll have to add
        // in the adjacent tile. Also the bottom left corner of the vine is too narrow.
        godot_error!("Couldn't land anywhere!");
        return None;
    }

    fn pick_adjacent_side(
        points: &PackedVector2Array,
        index: usize,
        next_pt_fn: fn(&PackedVector2Array, usize) -> usize,
        player_motion: Vector2,
    ) -> LandingSurface {
        let mut next_point_index = next_pt_fn(points, index);
        let a = points[index];
        let mut b = points[next_point_index];
        if a.distance_squared_to(b) < 1.0 {
            godot_print!("Distance from {a} to {b} is too small!");
            next_point_index = next_pt_fn(points, next_point_index);
            b = points[next_point_index];
            godot_print!("New point: {b}");
        }
        LandingSurface::new(a, b, player_motion)
    }
    fn check_collisions(&mut self) {
        if let Some(collision) = self
            .base_mut()
            .move_and_collide_ex(Vector2::ZERO)
            .test_only(true)
            .done()
        {
            godot_error!("Created a new collision!");
            print_collision(&collision);
        }
    }

    // Return whether there is enough room for the player to land on the surface.
    fn can_land_on_surface(&self, surface: &LandingSurface) -> bool {
        (surface.a - surface.b).length_squared() > pow(self.width() as f64, 2.0) as f32
    }

    fn bounding_box(&self) -> Rect2 {
        self.base()
            .get_node_as::<CollisionShape2D>("CollisionShape2D")
            .get_shape()
            .unwrap()
            .get_rect()
    }

    fn width(&self) -> f32 {
        self.bounding_box().size.x
    }

    fn height(&self) -> f32 {
        self.bounding_box().size.y
    }
}

// TODO: Probably better to return a reference? Or at least change the name?
fn next_point(points: &PackedVector2Array, i: usize) -> usize {
    assert!(i < points.len());
    if i == points.len() - 1 {
        0
    } else {
        i + 1
    }
}
// TODO: Same
fn prior_point(points: &PackedVector2Array, i: usize) -> usize {
    assert!(i < points.len());
    if i == 0 {
        points.len() - 1
    } else {
        i - 1
    }
}

fn print_collision(collision: &Gd<KinematicCollision2D>) {
    godot_print!("collision {:?}", collision);
    godot_print!("\tangle: {}", collision.get_angle());
    godot_print!("\tnormal: {}", collision.get_normal());
    godot_print!("\tcollider velocity: {}", collision.get_collider_velocity());
    godot_print!("\ttravel: {}", collision.get_travel());
    godot_print!("\tremainder: {}", collision.get_remainder());
    godot_print!("\tposition: {}", collision.get_position());
    godot_print!("\tdepth: {}", collision.get_depth());
    godot_print!("\tlocal shape: {:?}", collision.get_local_shape());
    godot_print!("\tcollider shape: {:?}", collision.get_collider_shape());
}

// Returns global coordinates.
fn get_collider_points(
    collider: Gd<Object>,
    collision_position: &Vector2,
) -> Option<PackedVector2Array> {
    // As of this writing, the player only detects collisions with the environment, i.e.
    // walls/ceilings/trees the player can land on. So this should always be a
    // `TileMapLayer`.
    if let Some(tile_map_layer) = collider.try_cast::<TileMapLayer>().ok() {
        let local_collision = tile_map_layer.to_local(*collision_position);
        let map_coordinates = tile_map_layer.local_to_map(local_collision);
        return get_collider_points_from_tile_map_layer(
            &tile_map_layer,
            map_coordinates,
            Some(local_collision),
        );
    } else {
        godot_error!("Collided with something other than a TileMapLayer??");
    }
    None
}

// Returns global coordinates.
fn get_collider_points_from_tile_map_layer(
    tile_map_layer: &Gd<TileMapLayer>,
    map_coordinates: Vector2i,
    local_collision: Option<Vector2>,
) -> Option<PackedVector2Array> {
    let local_tile_center = tile_map_layer.map_to_local(map_coordinates);
    if let Some(tile_data) = tile_map_layer.get_cell_tile_data(map_coordinates) {
        // This should correspond to the layer I've used in the editor.
        const LAYER_ID: i32 = 0;
        let count = tile_data.get_collision_polygons_count(LAYER_ID);
        if count > 1 {
            godot_error!("Need to handle {count} polygons in one tile!");
        }

        let mut points = tile_data.get_collision_polygon_points(LAYER_ID, 0);
        godot_print!("\t\t\tpolygon 0: {points}");
        for point in points.as_mut_slice() {
            let local_point = local_tile_center + *point;
            *point = tile_map_layer.to_global(local_point);
            godot_print!("\t\t\t\tglobal: {}", *point);
        }
        if local_collision.is_some() {
            // Check for collisions that are directly on the edge of the tile.
            // Note: This assumes square tiles.
            let mut next_tile_offset: Option<Vector2i> = None;
            let half_tile_width =
                tile_map_layer.get_tile_set().unwrap().get_tile_size().x as f32 / 2.0;
            if local_collision.unwrap().x == local_tile_center.x - half_tile_width {
                next_tile_offset = Some(Vector2i::LEFT);
            } else if local_collision.unwrap().y == local_tile_center.y - half_tile_width {
                next_tile_offset = Some(Vector2i::UP);
            }
            // Note: I haven't been able to reproduce landing on the right or
            // bottom edges of a tile, but the code should be similar.
            else if local_collision.unwrap().x == local_tile_center.x + half_tile_width {
                // Use `godot_error` just so it will stand out and I can create
                // a repro case.
                godot_error!("Found a repro case for right edge of tile?");
                next_tile_offset = Some(Vector2i::RIGHT);
            } else if local_collision.unwrap().y == local_tile_center.y + half_tile_width {
                // Use `godot_error` just so it will stand out and I can create
                // a repro case.
                godot_error!("Found a repro case for bottom edge of tile?");
                next_tile_offset = Some(Vector2i::DOWN);
            }
            if let Some(offset) = next_tile_offset {
                let next_map_coord = map_coordinates + offset;
                if let Some(next_points) =
                    get_collider_points_from_tile_map_layer(tile_map_layer, next_map_coord, None)
                {
                    let polygon_array =
                        Geometry2D::singleton().merge_polygons(&next_points, &points);
                    if polygon_array.len() == 1 {
                        godot_print!("have a new merged polygon!");
                        return Some(polygon_array.at(0));
                    } else {
                        godot_print!(
                            "Merge resulted in {} polygons: {polygon_array}",
                            polygon_array.len()
                        );
                    }
                } else {
                    godot_print!("\tcouldn't get tile data to the left!")
                }
            }
        }
        return Some(points);
    } else {
        return None;
    }
}
