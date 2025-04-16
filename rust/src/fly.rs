use godot::classes::{AnimatedSprite2D, Area2D, IArea2D};
use godot::global::{randf, randi_range};
use godot::prelude::*;

use crate::player::Player;

#[derive(Clone)]
enum State {
    Reset,               // Search for an idle route
    Idle,                // Fly between two points, determined in `Reset`
    Fleeing(Gd<Player>), // Fleeing a player trying to eat them
}

// Path for the fly to travel
#[derive(Default)]
struct Path {
    // These are in local coordinates.
    start: Vector2,
    end: Vector2,
    // If true, the fly is traveling back to the start
    reverse: bool,
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Fly {
    /// When `true`, code will drive the fly's motion.
    #[export]
    self_directed: bool,
    state: State,
    path: Path,
    #[export]
    movement_speed: f32,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for Fly {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            self_directed: false,
            state: State::Reset,
            path: Path::default(),
            movement_speed: 20.0,
            base,
        }
    }

    fn ready(&mut self) {
        let body_entered = self.base().callable("on_body_entered");
        self.base_mut().connect("body_entered", &body_entered);

        let mut sprite = self
            .base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        let frame_count = sprite
            .get_sprite_frames()
            .and_then(|frames| Some(frames.get_frame_count("default")))
            .unwrap();
        let frame = randi_range(0, (frame_count - 1).into()) as i32;
        sprite.set_frame_and_progress(frame, randf() as f32);
    }

    fn physics_process(&mut self, delta: f32) {
        if !self.self_directed {
            return;
        }

        match self.state.clone() {
            State::Reset => {
                // TODO: Pick a better path to fly between
                let start = self.base().get_position();
                let end = start + Vector2 { x: 64.0, y: 0.0 };
                self.path = Path {
                    start,
                    end,
                    reverse: false,
                };
                self.state = State::Idle;
            }
            State::Idle => {
                let dest = if self.path.reverse {
                    &self.path.start
                } else {
                    &self.path.end
                };
                let position = self.base().get_position();
                let movement_direction = *dest - position;
                let movement_delta = self.movement_speed * delta;
                let new_position = position.move_toward(*dest, movement_delta);
                if new_position == *dest {
                    self.path.reverse = !self.path.reverse;
                }
                let x_scale = if movement_direction.x >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                self.base_mut().set_scale(Vector2 { x: x_scale, y: 1.0 });
                self.base_mut().set_position(new_position);
            }
            State::Fleeing(_player) => todo!(),
        }
    }
}

#[godot_api]
impl Fly {
    #[signal]
    fn eaten();

    #[func]
    fn on_body_entered(&mut self, body: Gd<Node2D>) {
        if body.is_in_group("player") {
            self.base_mut().queue_free();
            self.base_mut().emit_signal("eaten", &[]);
        }
    }
}
