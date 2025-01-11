use crate::alligator::Alligator;
use godot::classes::{CharacterBody2D, InputEvent, InputEventKey, InputEventScreenTouch, Node2D};
use godot::global::Key;
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
        if let Some(touch_event) = event.clone().try_cast::<InputEventScreenTouch>().ok() {
            if touch_event.is_pressed() {
                self.fake_player().set_position(touch_event.get_position());
            }
        }
        if let Some(key) = event.try_cast::<InputEventKey>().ok() {
            if key.is_pressed() {
                self.animate_from_key(key.get_keycode());
            }
        }
    }
}

impl TestAlligator {
    fn fake_player(&self) -> Gd<CharacterBody2D> {
        self.base().get_node_as::<CharacterBody2D>("FakePlayer")
    }

    fn alligator(&self) -> Gd<Alligator> {
        self.base().get_node_as::<Alligator>("Alligator")
    }

    fn animate_from_key(&self, key: Key) {
        let anim = match key {
            Key::B => "blink",
            Key::F => "flash_eyes",
            Key::Q => "default",
            Key::R => "raise_eyebrows",
            Key::S => "shift_eyes",
            _ => return,
        };
        self.alligator().bind_mut().animate(anim, true);
    }
}
