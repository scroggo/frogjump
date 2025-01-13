use std::f32::consts::PI;
use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::jump_handler::{JumpDetector, JumpHandler};
use godot::classes::{
    AnimatedSprite2D, CharacterBody2D, ICharacterBody2D, InputEvent, InputEventScreenTouch, Timer,
};
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

/// Direction the player is facing.
#[derive(PartialEq, GodotConvert, Var, Export)]
#[godot(via=GString)]
enum Direction {
    Left,
    Right,
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
        self.base()
            .get_node_as::<Timer>("BlinkTimer")
            .connect("timeout", &blink);
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
        let motion = self.target_velocity * delta as f32;
        let collision_opt = self.base_mut().move_and_collide(motion);
        if let Some(collision) = collision_opt {
            if let Some(_collider) = collision.get_collider() {
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
                        self.sprite().set_flip_v(false);
                        self.sprite().set_flip_h(false);
                    }
                    Vector2 { x, y: _ } if x < -0.5 => {
                        self.direction = Direction::Left;
                        self.base_mut().set_rotation(PI / 2.0);
                        self.sprite().set_flip_v(true);
                        self.sprite().set_flip_h(false);
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
                    self.sprite().set_flip_v(false);
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

    #[func]
    fn on_blink_timeout(&self) {
        let mut sprite = self.sprite();
        if self.on_surface && !sprite.is_playing() {
            sprite.play_ex().name("blink").done();
        }
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
}
