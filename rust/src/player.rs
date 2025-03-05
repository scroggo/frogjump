use std::collections::HashSet;
use std::f32::consts::PI;
use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::jump_handler::{JumpDetector, JumpHandler};
use crate::landing_surface::LandingSurface;
use crate::math;
use godot::classes::{
    AnimatedSprite2D, CharacterBody2D, CollisionShape2D, Geometry2D, ICharacterBody2D, InputEvent,
    InputEventScreenTouch, KinematicCollision2D, Os, TileMapLayer, Timer,
};
use godot::global::{absf, cos, randf_range};
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

// The player's width works well for collisions, but make it a little bit
// smaller so that the player can land on surfaces that have enough room for the
// player's body but do for their feet.
const WIDTH_MODIFIER: f32 = 0.7;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    #[export]
    direction: Direction,
    #[export]
    target_velocity: Vector2,
    /// Initial speed when releasing the jump button at full strength.
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
        let os = Os::singleton();
        if os.has_feature("debug") {
            godot_print!("Running a debug build.");
        }
        if os.has_feature("release") {
            godot_print!("Running a release build.");
        }
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
            let new_position = match (shimmy_dest - old_position).try_normalized() {
                Some(direction) => {
                    let movement = direction * self.shimmy_speed * delta as f32;
                    let possible_position = old_position + movement;
                    let distance_squared_from_dest =
                        possible_position.distance_squared_to(shimmy_dest);
                    if distance_squared_from_dest < 1.0 {
                        // Avoid overshooting the destination. Note: This works because
                        // delta is small.
                        shimmy_dest
                    } else {
                        possible_position
                    }
                }
                None => shimmy_dest,
            };
            if new_position == shimmy_dest {
                self.shimmy_dest = None;
                self.sprite().play_ex().name("default").done();
            }
            self.base_mut().set_position(new_position);
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

                if collision.get_depth() > 0.0 {
                    // The player is penetrating the wall. Move back along the
                    // direction of motion far enough to remove the overlap.
                    if let Some(motion_normalized) = motion.try_normalized() {
                        let reverse_motion = -motion_normalized;
                        let depth_vector = collision.get_depth() * collision.get_normal();
                        let offset = depth_vector.length()
                            / cos(reverse_motion.angle_to(depth_vector).into()) as f32;
                        let position = self.base().get_position();
                        self.base_mut()
                            .set_position(position + offset * reverse_motion);
                    }
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
                if let Some(surface) = landing_surface {
                    // When landing on a corner, `a` represents the corner.
                    // TODO: This is totally arbitrary. Enforce/make clearer.
                    if collision_position == surface.a {
                        // Land on the corner directly, pushed away by the
                        // normal. If this is too jarring in some cases, we can
                        // try starting from the player's position.
                        let global_position = surface.a + normal * (self.height() / 2.0);
                        let new_player_position = self.to_local_position(global_position);
                        self.base_mut().set_position(new_player_position);

                        // Shimmy more fully onto the surface over the next
                        // several frames.
                        if let Some(surface_direction) = (surface.b - surface.a).try_normalized() {
                            let motion = (self.width() / 2.0) * surface_direction * WIDTH_MODIFIER;
                            if self.would_collide(motion) {
                                godot_print!("Shimmying (corner) would cause collisions!");
                            } else {
                                self.shimmy_dest = Some(new_player_position + motion);
                                self.sprite().play_ex().name("shimmy").done();
                            }
                        }
                    } else {
                        // Line up the player so that they appear to be resting directly
                        // on the surface.

                        // Need the global position for the player, since the surface
                        // uses global positions.
                        let global_position = self.get_global_position();

                        // We have the normal for the plane, we just need its
                        // distance from the origin.
                        let d = normal.dot(surface.a);

                        let distance_to_surface = normal.dot(global_position) - d;
                        let desired_distance = self.height() / 2.0;
                        let dist_to_move = desired_distance - distance_to_surface;
                        let new_player_position =
                            self.base().get_position() + dist_to_move * normal;
                        self.base_mut().set_position(new_player_position);

                        // Shimmy onto the surface, if needed.
                        if !self.can_land_on_surface(&surface) {
                            // TODO: Shimmy around a corner?
                            godot_print!("Don't fit on surface!");
                        } else {
                            if let Some(shimmy_dest) = self.find_shimmy_dest(&surface) {
                                let motion = shimmy_dest - self.base().get_position();
                                if self.would_collide(motion) {
                                    godot_print!("Shimmying would cause collisions!");
                                } else {
                                    self.shimmy_dest = Some(shimmy_dest);
                                    self.sprite().play_ex().name("shimmy").done();
                                }
                            }
                        }
                    }
                    godot_print!("Player's local position: {}", self.base().get_position());
                    if self.would_collide(Vector2::ZERO) {
                        godot_error!("Created a new collision!");
                    }
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

                    // Sometimes the rotation causes new collisions. (The change
                    // to `flip_h` should make no difference because the
                    // collision rectangle is centered around the player.)
                    // Remove any overlap.
                    let position = self.base().get_position();
                    let bb = self.bounding_box();

                    let mut base_mut = self.base_mut();
                    if let Some(collision) = base_mut
                        .move_and_collide_ex(Vector2::ZERO)
                        .test_only(true)
                        .done()
                    {
                        // Since this is an unusual collision - the rectangle
                        // instantaneously changed - the depth is incorrect.
                        // But landing on this surface previously placed the
                        // player height/2 away from the surface, and now they
                        // should be width/2 away, so add the difference.
                        let diff = (bb.size.x - bb.size.y) / 2.0;
                        let offset = collision.get_normal() * (collision.get_depth() + diff);
                        let new_position = position + offset;
                        base_mut.set_position(new_position);
                    }
                }
                if self.would_collide(Vector2::ZERO) {
                    godot_error!("Shouldn't still have a collision!");
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

    fn get_global_position(&self) -> Vector2 {
        self.base()
            .get_parent()
            .unwrap()
            .cast::<Node2D>()
            .to_global(self.base().get_position())
    }

    fn to_local_position(&self, global_position: Vector2) -> Vector2 {
        self.base()
            .get_parent()
            .unwrap()
            .cast::<Node2D>()
            .to_local(global_position)
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
            let mut a = points[i];
            let mut b = points[i2];

            // Construct a plane from a normal and a point. See
            // https://docs.godotengine.org/en/stable/tutorials/math/vectors_advanced.html#constructing-a-plane-in-2d
            let n_opt = math::normal(a, b, player_motion);
            if n_opt.is_none() {
                // This side is too small. Skip it.
                continue;
            }
            let mut n = n_opt.unwrap();
            let d = n.dot(a);

            // If the collision is in this plane and the normal matches, this is
            // the side we landed on.
            let distance = n.dot(collision_position) - d;
            const TOLERANCE: f64 = 0.2;
            if absf(distance as f64) < TOLERANCE {
                godot_print!("Collision {collision_position} is {distance} from side {a}-{b}");
                let dot_product = n.dot(collision_normal);
                if ((1.0 - dot_product) as f64) < TOLERANCE {
                    // Found the proper side. Check adjacent sides in case it's
                    // at a tiny angle that we can treat as a long surface.
                    {
                        let i3 = next_point(points, i2);
                        let c = points[i3];
                        if let Some(bc) = LandingSurface::new(b, c, player_motion) {
                            if math::same_normals_approx(n, bc.normal) {
                                godot_print!("Appending bc!");
                                b = c;
                                n = bc.normal;
                            }
                        }
                    }
                    {
                        let i0 = prior_point(points, i);
                        let z = points[i0];
                        if let Some(za) = LandingSurface::new(z, a, player_motion) {
                            if math::same_normals_approx(za.normal, n) {
                                godot_print!("Appending za!");
                                a = z;
                                n = za.normal;
                            }
                        }
                    }
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
        if math::same_normals_approx(landing_surface_a.normal, landing_surface_b.normal) {
            godot_print!("Treat two sides as same surface!");
            if let Some(landing_surface_full) =
                LandingSurface::new(landing_surface_a.b, landing_surface_b.b, player_motion)
            {
                if !self.can_land_on_surface(&landing_surface_full) {
                    godot_error!("Surfaces are too small! Landing anyway!");
                }
                return Some(landing_surface_full);
            }
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
                // The surface picked the normal based on the player motion,
                // which I would expect to generally work - the player must be
                // colliding from outside the polygon. But corners are funny.
                // We might pick a side such that the player's motion looks to
                // come from inside.
                return Some(surface.correct_normal(points));
            }
        }
        // TODO: We might hit this case if you land close to the branch - then we'll have to add
        // in the adjacent tile.
        godot_error!("Couldn't land anywhere!");
        return None;
    }

    fn pick_adjacent_side(
        points: &PackedVector2Array,
        index: usize,
        next_pt_fn: fn(&PackedVector2Array, usize) -> usize,
        player_motion: Vector2,
    ) -> LandingSurface {
        let next_point_index = next_pt_fn(points, index);
        let a = points[index];
        let b = points[next_point_index];
        // `smooth_polygon` already checked the distance between these two points.
        LandingSurface::new(a, b, player_motion).expect("surface should have a normal!")
    }

    fn would_collide(&mut self, motion: Vector2) -> bool {
        let mut bb = self.bounding_box();
        bb.position = bb.position + self.get_global_position();
        let bb2 = Rect2 {
            position: bb.position + motion,
            size: bb.size,
        };
        godot_print!("would_collide? test for collisions with motion {motion}");
        godot_print!("\tcurrent position: {bb}");
        godot_print!("\tafter movement:   {bb2}");

        if let Some(collision) = self
            .base_mut()
            .move_and_collide_ex(motion)
            .test_only(true)
            .done()
        {
            godot_print!("\tYes!");
            print_collision(&collision);
            return true;
        }
        godot_print!("\tNo");
        false
    }

    // Return whether there is enough room for the player to land on the surface.
    fn can_land_on_surface(&self, surface: &LandingSurface) -> bool {
        surface.length_squared() > math::squared(self.width() * WIDTH_MODIFIER)
    }

    // If the player is hanging off one edge of the surface or the other, return
    // the location they should shimmy to.
    fn find_shimmy_dest(&self, surface: &LandingSurface) -> Option<Vector2> {
        if let Some(shimmy_dest) =
            self.find_shimmy_dest_internal(surface.a, surface.b, surface.normal)
        {
            return Some(shimmy_dest);
        }
        if let Some(shimmy_dest) =
            self.find_shimmy_dest_internal(surface.b, surface.a, surface.normal)
        {
            return Some(shimmy_dest);
        }
        None
    }

    fn find_shimmy_dest_internal(&self, a: Vector2, b: Vector2, n: Vector2) -> Option<Vector2> {
        // The bottom middle of the player, which should be in the plane of the surface.
        let current_position = self.get_global_position();

        let player_middle = current_position - n * self.height() / 2.0;

        let surface_direction = (a - b).try_normalized()?;
        let bottom_corner = player_middle + surface_direction * self.width() / 2.0;
        let v = a - bottom_corner;
        let dot_product = v.try_normalized()?.dot(surface_direction);

        // Since these are normalized, -1 would be exactly pointing in the
        // opposite direction. Use a small tolerance.
        if dot_product < -0.95 {
            // Player is overhanging the surface in the direction of `a`. Move
            // the other way to be on the surface.
            return Some(self.to_local_position(current_position + v * WIDTH_MODIFIER));
        }
        None
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
        if let Some(mut points) = get_collider_points_from_tile_map_layer(
            &tile_map_layer,
            map_coordinates,
            Some(local_collision),
        ) {
            smooth_polygon(&mut points);
            return Some(points);
        }
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
    let mut points_so_far: Option<PackedVector2Array> = None;
    let half_tile_width = tile_map_layer.get_tile_set().unwrap().get_tile_size().x as f32 / 2.0;

    // Extend in these directions as needed.
    let mut directions_to_add: HashSet<Vector2i> = HashSet::new();
    if let Some(tile_data) = tile_map_layer.get_cell_tile_data(map_coordinates) {
        // This should correspond to the layer I've used in the editor.
        const LAYER_ID: i32 = 0;
        let count = tile_data.get_collision_polygons_count(LAYER_ID);
        if count > 1 {
            godot_error!("Need to handle {count} polygons in one tile!");
        }

        let mut points = tile_data.get_collision_polygon_points(LAYER_ID, 0);

        // Check for edges that align with the tile edges. This indicates that
        // the collision shape may extend onto the next tile. Use presence of
        // `local_collision`, which is only set for the first call to this
        // method (i.e. it is `None` in recursive calls).
        if local_collision.is_some() {
            for i in 0..points.len() {
                let a = points[i];
                let b = points[next_point(&points, i)];
                if a.x == b.x {
                    if a.x == half_tile_width {
                        directions_to_add.insert(Vector2i::RIGHT);
                    } else if a.x == -half_tile_width {
                        directions_to_add.insert(Vector2i::LEFT);
                    }
                }
                if a.y == b.y {
                    if a.y == half_tile_width {
                        directions_to_add.insert(Vector2i::DOWN);
                    } else if a.y == -half_tile_width {
                        directions_to_add.insert(Vector2i::UP);
                    }
                }
            }
        }
        godot_print!("\t\t\tpolygon 0: {points}");
        for point in points.as_mut_slice() {
            let local_point = local_tile_center + *point;
            *point = tile_map_layer.to_global(local_point);
            godot_print!("\t\t\t\tglobal: {}", *point);
        }
        points_so_far = Some(points);
    }
    if local_collision.is_some() {
        // Check for collisions that are directly on the edge of the tile. This
        // catches some cases missed above. e.g. If a tile is empty, and the
        // collision occurs on the edge, Godot may pick the empty tile first.
        // These checks ensure we include the adjacent tile with a collision
        // polygon.
        // Note: This assumes square tiles.

        if local_collision.unwrap().x == local_tile_center.x - half_tile_width {
            directions_to_add.insert(Vector2i::LEFT);
        } else if local_collision.unwrap().y == local_tile_center.y - half_tile_width {
            directions_to_add.insert(Vector2i::UP);
        }
        // Note: I haven't been able to reproduce landing on the right or
        // bottom edges of a tile, but the code should be similar.
        else if local_collision.unwrap().x == local_tile_center.x + half_tile_width {
            // Use `godot_error` just so it will stand out and I can create
            // a repro case.
            godot_error!("Found a repro case for right edge of tile?");
            directions_to_add.insert(Vector2i::RIGHT);
        } else if local_collision.unwrap().y == local_tile_center.y + half_tile_width {
            // Use `godot_error` just so it will stand out and I can create
            // a repro case.
            godot_error!("Found a repro case for bottom edge of tile?");
            directions_to_add.insert(Vector2i::DOWN);
        }

        for offset in directions_to_add {
            let next_map_coord = map_coordinates + offset;
            if let Some(next_points) =
                get_collider_points_from_tile_map_layer(tile_map_layer, next_map_coord, None)
            {
                if points_so_far.is_none() {
                    // This can only be reached if the collision is directly on
                    // the edge. We wouldn't want to check tiles adjacent to the
                    // empty tile anyway, since they would not be adjacent to
                    // this one.
                    // Note: With some hypothetical tiles/collision polygons not
                    // in our current tile set, we might need to add tiles
                    // adjacent to *this* one.
                    return Some(next_points);
                }
                let points = points_so_far.unwrap();
                let polygon_array = Geometry2D::singleton().merge_polygons(&next_points, &points);
                if polygon_array.len() == 1 {
                    godot_print!("have a new merged polygon!");
                    points_so_far = Some(polygon_array.at(0));
                } else {
                    godot_print!(
                        "Merge resulted in {} polygons: {polygon_array}",
                        polygon_array.len()
                    );
                    // This likely means that the next tile over had a
                    // disconnected polygon. Ignore it. Restore `points_so_far`,
                    // which was unwrapped for `merge_polygons` above.
                    points_so_far = Some(points);
                }
            }
        }
    }
    return points_so_far;
}

// Modify the supplied polygon to remove unnecessary points.
// Tile data may not line up perfectly, resulting in e.g. two points
// that are right next to each other.
// Note: Not exhaustive for all hypothetical polygons. This is
// intended specifically to catch tiles not lining up.
fn smooth_polygon(polygon: &mut PackedVector2Array) {
    let mut stack = Vec::new();
    for i in 0..polygon.len() {
        let i2 = next_point(&polygon, i);
        let a = polygon[i];
        let b = polygon[i2];
        if a.distance_squared_to(b) < 1.0 {
            godot_print!("Removing {b}!");
            stack.push(i2);
        }
    }

    // Remove in reverse order so the indices are still correct.
    while let Some(index) = stack.pop() {
        polygon.remove(index);
    }
}
