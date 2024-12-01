use std::f32::consts::PI;

use godot::classes::{AnimatedSprite2D, CharacterBody2D, ICharacterBody2D, Timer};
use godot::prelude::*;

/// Direction the player is facing.
#[derive(PartialEq, GodotConvert, Var, Export)]
#[godot(via=GString)]
enum Direction {
    Left,
    Right,
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
    on_surface: bool, // True when on any surface: floor, wall, ceiling.
    on_ceiling: bool,
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
    }

    fn physics_process(&mut self, delta: f64) {
        if !self.on_surface {
            self.target_velocity.y += delta as f32 * self.fall_acceleration;
        }
        let motion = self.target_velocity * delta as f32;
        let collision_opt = self.base_mut().move_and_collide(motion);
        if let Some(collision) = collision_opt {
            if let Some(collider) = collision.get_collider() {
                if collider.cast::<Node>().is_in_group("wall") {
                    self.on_surface = true;

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
                    // Note: Assumes just vertical (and horizontal) walls for now.
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
                            self.on_ceiling = true;
                            self.base_mut().set_rotation(PI);
                            let flip_h = self.direction == Direction::Left;
                            self.sprite().set_flip_h(flip_h);
                        }
                        _ => (),
                    }
                }
            }
        }

        if self.on_surface {
            let input = Input::singleton();
            if input.is_action_pressed("jump") {
                // Always jump right-side up, facing `Direction`.
                self.base_mut().set_rotation(0.0);
                let flip_h = self.direction == Direction::Right;
                self.sprite().set_flip_h(flip_h);
                self.sprite().set_flip_v(false);

                self.target_velocity = self.get_jump();
                self.sprite().play_ex().name("jump").done();
                self.on_surface = false;
                self.on_ceiling = false;
            } else {
                self.target_velocity = Vector2::ZERO;
            }
        }
    }
}

#[godot_api]
impl Player {
    fn get_jump(&self) -> Vector2 {
        // TODO: Fill up a power meter for variable strength jumps.
        let ceiling_multiplier = match self.on_ceiling {
            true => 1.0,
            false => -1.0,
        };
        let jump_strength = self.max_jump_strength / 2.0 * ceiling_multiplier;
        const PI_OVER_FOUR: f32 = PI / 4.0;
        let jump_angle = match self.direction {
            Direction::Left => PI_OVER_FOUR,
            Direction::Right => -PI_OVER_FOUR,
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
}
