use godot::classes::{CharacterBody2D, InputEvent, InputEventScreenTouch, Node2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct TestAlligator {
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TestAlligator {
    fn init(base: Base<Node2D>) -> Self {
        Self { base }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Some(touch_event) = event.try_cast::<InputEventScreenTouch>().ok() {
            if touch_event.is_pressed() {
                self.fake_player().set_position(touch_event.get_position());
            }
        }
    }
}

impl TestAlligator {
    fn fake_player(&self) -> Gd<CharacterBody2D> {
        self.base().get_node_as::<CharacterBody2D>("FakePlayer")
    }
}
