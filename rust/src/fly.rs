use godot::classes::{AnimatedSprite2D, Area2D, IArea2D};
use godot::global::{randf, randi_range};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Fly {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for Fly {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
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
