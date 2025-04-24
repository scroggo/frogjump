use crate::direction::Direction;
use godot::classes::{AnimatedSprite2D, Node2D, Timer};
use godot::global::randf_range;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct Toucan {
    #[export]
    direction: Direction,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Toucan {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            direction: Direction::Right,
            base,
        }
    }

    fn ready(&mut self) {
        let callback = self.base().callable("on_timer_timeout");
        let mut timer = self.timer();
        timer.connect("timeout", &callback);
        self.start_timer();
    }
}

#[godot_api]
impl Toucan {
    #[func]
    fn on_timer_timeout(&mut self) {
        let anim = match self.direction {
            Direction::Left => "turn_head_right",
            Direction::Right => "turn_head_left",
        };
        self.sprite().play_ex().name(anim).done();
        let new_direction = !self.direction;
        self.direction = new_direction;
        self.start_timer();
    }

    fn timer(&self) -> Gd<Timer> {
        self.base().get_node_as::<Timer>("Timer")
    }

    fn start_timer(&self) {
        let wait_time = randf_range(3.0, 7.0);
        self.timer().start_ex().time_sec(wait_time).done();
    }

    fn sprite(&self) -> Gd<AnimatedSprite2D> {
        self.base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D")
    }
}
