use godot::classes::InputEvent;
use godot::prelude::*;

/// This class simply marks ENTER/"NEXT" as handled so its level cannot be
/// skipped.
#[derive(GodotClass)]
#[class(base=Node)]
struct StealEnter {
    base: Base<Node>,
}

#[godot_api]
impl INode for StealEnter {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("NEXT") {
            self.base().get_viewport().unwrap().set_input_as_handled();
        }
    }
}
