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

    fn ready(&mut self) {
        let on_player_eaten = self.base().callable("on_player_eaten");
        self.alligator().connect("player_eaten", &on_player_eaten);
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

#[godot_api]
impl TestAlligator {
    fn fake_player(&mut self) -> Gd<CharacterBody2D> {
        // This name matches the name set in the editor. The editor version is
        // arguably redundant, since we'll just create a new one if it's not in
        // the editor (or has a different name!), but it demonstrates where the
        // node will go.
        let name = "FakePlayer";
        if let Some(fake_player) = self.base().try_get_node_as::<CharacterBody2D>(name) {
            return fake_player;
        }
        // Respawn.
        let scene = load::<PackedScene>("res://test_scenes/fake_player.tscn");
        let mut fake_player = scene.instantiate().unwrap().cast::<CharacterBody2D>();
        fake_player.set_name(name);
        self.base_mut().add_child(&fake_player);
        fake_player
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

    #[func]
    fn on_player_eaten(&mut self, mut player: Gd<Node2D>) {
        if let Some(mut parent) = player.get_parent() {
            parent.remove_child(&player);
        } else {
            godot_error!("Attempting to eat player not in tree?");
        }
        player.queue_free();
    }
}
