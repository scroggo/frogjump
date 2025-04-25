use crate::direction::Direction;

use godot::classes::{AnimatedSprite2D, Area2D, Engine, IArea2D, RayCast2D};
use godot::global::{randf, randi_range};
use godot::prelude::*;

#[derive(Clone)]
enum State {
    Callibrating,        // Looking for a surface to hover over.
    Hovering,            // Hovering over a surface.
}

#[derive(GodotClass)]
#[class(base=Area2D, tool)]
struct Fly {
    /// When `true`, code will drive the fly's motion.
    #[export]
    self_directed: bool,
    state: State,
    #[export]
    direction: Direction,
    #[export]
    movement_speed: f32,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for Fly {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            self_directed: true,
            state: State::Callibrating,
            direction: Direction::Right,
            movement_speed: 100.0,
            base,
        }
    }

    fn ready(&mut self) {
        self.set_facing();
        if Engine::singleton().is_editor_hint() {
            return;
        }

        let gd = Gd::from_instance_id(self.base().instance_id());
        self.base_mut()
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::on_body_entered);

        let mut avoidance_area = self
            .base()
            .get_node_as::<Area2D>("CollisionAvoidanceArea2D");
        avoidance_area
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::on_collision_avoidance);

        let mut sprite = self
            .base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        let frame_count = sprite
            .get_sprite_frames()
            .and_then(|frames| Some(frames.get_frame_count("default")))
            .unwrap();
        let frame = randi_range(0, (frame_count - 1).into()) as i32;
        sprite.set_frame_and_progress(frame, randf() as f32);

        if !self.self_directed {
            self.ray_cast().set_enabled(false);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        if Engine::singleton().is_editor_hint() {
            return;
        }
        if !self.self_directed {
            return;
        }

        let ray_cast = self.ray_cast();
        match self.state.clone() {
            State::Callibrating => {
                // During the first couple of frames, the ray cast may not
                // succeed. Continue to hover in the faced direction.
                self.move_internal(delta);

                if ray_cast.is_colliding() {
                    self.state = State::Hovering;
                }
            }
            State::Hovering => {
                if !ray_cast.is_colliding() {
                    // Flew off the edge.
                    self.turn_around();
                }
                self.move_internal(delta);
            }
        }
    }
}

#[godot_api]
impl Fly {
    #[signal]
    fn eaten();

    #[func]
    fn on_body_entered(&mut self, _body: Gd<Node2D>) {
        // This `Area2D` only detects the player.
        self.base_mut().queue_free();
        self.base_mut().emit_signal("eaten", &[]);
    }

    #[func]
    fn on_collision_avoidance(&mut self, _body: Gd<Node2D>) {
        // This callback is only called by collisions with the world.
        self.turn_around();
    }

    fn direction_multipler(&self) -> f32 {
        if self.direction == Direction::Right {
            1.0
        } else {
            -1.0
        }
    }

    fn set_facing(&mut self) {
        let x_scale = self.direction_multipler();
        self.base_mut().set_scale(Vector2 { x: x_scale, y: 1.0 });
    }

    fn turn_around(&mut self) {
        self.direction = !self.direction;
        // Switch back to `Callibrating` to avoid switching back and forth.
        self.state = State::Callibrating;
        self.set_facing();
    }

    fn move_internal(&mut self, delta: f64) {
        let movement_delta = self.movement_speed as f64 * delta * self.direction_multipler() as f64;
        let position = self.base().get_position();
        self.base_mut().set_position(
            position
                + Vector2 {
                    x: movement_delta as f32,
                    y: 0.0,
                },
        );
    }

    fn ray_cast(&self) -> Gd<RayCast2D> {
        self.base().get_node_as::<RayCast2D>("RayCast2D")
    }
}
