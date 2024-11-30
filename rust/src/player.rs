use std::f32::consts::PI;

use godot::classes::{AnimatedSprite2D, CharacterBody2D, ICharacterBody2D, Timer};
use godot::prelude::*;

/// Direction the player is facing.
#[derive(PartialEq)]
enum Direction {
    Left,
    Right,
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct Player {
    direction: Direction,
    target_velocity: Vector2,
    #[export]
    max_jump_strength: f32,
    #[export]
    fall_acceleration: f32,
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
            base,
        }
    }

    fn ready(&mut self) {
        let blink = self.base().callable("on_blink_timeout");
        self.base()
            .get_node_as::<Timer>("BlinkTimer")
            .connect("timeout", &blink);
    }

    fn physics_process(&mut self, delta: f64) {
        if !self.on_surface() {
            self.target_velocity.y += delta as f32 * self.fall_acceleration;
        }
        let target_velocity = self.target_velocity;
        self.base_mut().set_velocity(target_velocity);
        self.base_mut().move_and_slide();

        for index in 0..self.base().get_slide_collision_count() {
            let collision = self.base_mut().get_slide_collision(index).unwrap();
            if collision.get_collider().is_none() {
                // TODO: Remove. So far I have not seen this log.
                godot_print!("How was collider {index} none?");
                continue;
            }

            let collider = collision.get_collider().unwrap();
            if collider.cast::<Node>().is_in_group("wall") {
                // Without this check, we would keep restarting the animation
                // since it's still colliding while touching.
                if self.target_velocity.length_squared() > 0.0 {
                    // Reverse the jump animation to land.
                    // TODO: Player starts off falling. Since they're not in
                    // the jumping pose, it looks awkward to play this
                    // animation. But the real solution may just be to not fall
                    // like that.
                    self.sprite()
                        .play_ex()
                        .name("jump")
                        .custom_speed(-3.0)
                        .from_end(true)
                        .done();
                }
                // Note: Assumes just vertical walls for now.
                match collision.get_normal() {
                    Vector2 { x, y: _ } if x > 0.0 => {
                        self.direction = Direction::Right;
                        self.base_mut().set_rotation(PI / 2.0);
                        self.sprite().set_flip_v(false);
                        self.sprite().set_flip_h(false);
                    }
                    Vector2 { x, y: _ } if x < 0.0 => {
                        self.direction = Direction::Left;
                        self.base_mut().set_rotation(PI / 2.0);
                        self.sprite().set_flip_v(true);
                        self.sprite().set_flip_h(false);
                    }
                    Vector2 { x: _, y } if y > 0.0 => {
                        // TODO: Verify this works as expected.
                        self.sprite().set_flip_v(true);

                        // Jumping sets flip_h properly, so no need to adjust
                        // here.
                    }
                    _ => (),
                }
            }
        }

        if self.on_surface() {
            let input = Input::singleton();
            if input.is_action_pressed("jump") {
                // Always jump right-side up, facing `Direction`.
                self.base_mut().set_rotation(0.0);
                let flip_h = self.direction == Direction::Right;
                self.sprite().set_flip_h(flip_h);
                self.sprite().set_flip_v(false);

                // TODO: Fill up a power meter for variable strength jumps.
                self.target_velocity =
                    Vector2::new(0.0, -self.max_jump_strength / 2.0).rotated(self.get_jump_angle());
                self.sprite().play_ex().name("jump").done();
            } else {
                self.target_velocity = Vector2::ZERO;
            }
        }
    }
}

#[godot_api]
impl Player {
    fn get_jump_angle(&self) -> f32 {
        const PI_OVER_FOUR: f32 = PI / 4.0;
        match self.direction {
            Direction::Left => -PI_OVER_FOUR,
            Direction::Right => PI_OVER_FOUR,
        }
    }

    #[func]
    fn on_blink_timeout(&self) {
        let mut sprite = self.sprite();
        if self.on_surface() && !sprite.is_playing() {
            sprite.play_ex().name("blink").done();
        }
    }

    fn on_surface(&self) -> bool {
        self.base().is_on_floor() || self.base().is_on_wall() || self.base().is_on_ceiling()
    }

    fn sprite(&self) -> Gd<AnimatedSprite2D> {
        self.base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D")
    }
}
