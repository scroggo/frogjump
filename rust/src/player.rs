use std::f32::consts::PI;
use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::jump_handler::{JumpDetector, JumpHandler};
use godot::classes::{
    AnimatedSprite2D, CharacterBody2D, ICharacterBody2D, InputEvent, InputEventScreenTouch,
    KinematicCollision2D, TileMapLayer, Timer,
};
use godot::global::cos;
use godot::global::randf_range;
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
    pos: Vector2,
    vel: Vector2,
    dir: Direction,
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
        if !self.on_surface {
            self.target_velocity.y += delta as f32 * self.fall_acceleration;
        }
        let old_position = self.base().get_position();
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
                if collision.get_depth() > 0.0  && !motion.is_zero_approx() {
                    // The player is penetrating the wall. Move back along the
                    // direction of motion far enough to remove the overlap.
                    let reverse_motion = -motion.normalized();
                    let depth_vector = collision.get_depth() * collision.get_normal();
                    let offset = depth_vector.length() / cos(reverse_motion.angle_to(depth_vector).into()) as f32;
                    let position = self.base().get_position();
                    self.base_mut().set_position(position + offset * reverse_motion);
                    godot_print!("Moving back by {offset} along {reverse_motion} for change of {} from {position} to {}",
                        offset * reverse_motion, self.base().get_position());
                }
                let collision_position = collision.get_position();
                if let Some(points) = get_collider_points(collider, &collision_position) {
                    godot_print!("Returned points: {points}");
                    if points.contains(collision_position) {
                        godot_print!("hit a corner!");
                        // TODO: If we hit the corner exactly, we should pick a side and behave
                        // similarly as if we landed directly on that side.
                    }
                }
                self.on_surface = true;

                // Reverse the jump animation to land.
                self.sprite()
                    .play_ex()
                    .name("jump")
                    .custom_speed(-3.0)
                    .from_end(true)
                    .done();
                // Note: Assumes just vertical (and horizontal) walls for now.
                match collision.get_normal() {
                    Vector2 { x, y: _ } if x > 0.5 => {
                        self.direction = Direction::Right;
                        self.base_mut().set_rotation(PI / 2.0);
                        self.sprite().set_flip_h(false);
                    }
                    Vector2 { x, y: _ } if x < -0.5 => {
                        self.direction = Direction::Left;
                        self.base_mut().set_rotation(-PI / 2.0);
                        self.sprite().set_flip_h(true);
                    }
                    Vector2 { x: _, y } if y > 0.5 => {
                        self.on_ceiling = true;
                        self.base_mut().set_rotation(PI);
                        let flip_h = self.direction == Direction::Left;
                        self.sprite().set_flip_h(flip_h);
                    }
                    Vector2 { x: _, y } if y < -0.5 => {
                        self.base_mut().set_rotation(0.0);
                        let flip_h = self.direction == Direction::Right;
                        self.sprite().set_flip_h(flip_h);
                    }
                    normal => {
                        godot_error!("Landed with surprise normal {normal}");
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

fn get_collider_points(collider: Gd<Object>, collision_position: &Vector2) -> Option<PackedVector2Array> {
    // As of this writing, the player only detects collisions with the environment, i.e.
    // walls/ceilings/trees the player can land on. So this should always be a
    // `TileMapLayer`.
    if let Some(tile_map_layer) = collider.try_cast::<TileMapLayer>().ok() {
        let local_collision = tile_map_layer.to_local(*collision_position);
        let map_coordinates = tile_map_layer.local_to_map(local_collision);
        let local_tile_center = tile_map_layer.map_to_local(map_coordinates);
        godot_print!("\tcell {map_coordinates}");
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
            return Some(points);
        } else {
            godot_error!("\t\tno tile data??");
        }
    } else {
        godot_error!("Collided with something other than a TileMapLayer??");
    }
    None
}
