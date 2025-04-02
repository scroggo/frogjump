use godot::classes::{Area2D, IArea2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Area2D)]
struct DragonFly {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for DragonFly {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }
    fn ready(&mut self) {
        let body_entered = self.base().callable("on_body_entered");
        self.base_mut().connect("body_entered", &body_entered);
    }
}

#[godot_api]
impl DragonFly {
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
