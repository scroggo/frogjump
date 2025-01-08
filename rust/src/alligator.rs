use godot::classes::{AnimatedSprite2D, InputEvent, InputEventKey, Node2D, Timer};
use godot::global::{randf, randf_range, Key};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct Alligator {
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Alligator {
    fn init(base: Base<Node2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        let idle_timeout = self.base().callable("on_idle_timeout");
        self.base()
            .get_node_as::<Timer>("IdleTimer")
            .connect("timeout", &idle_timeout);
        self.start_idle_timer();
    }

    // For testing animations.
    // TODO: Remove
    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Some(key) = event.try_cast::<InputEventKey>().ok() {
            if key.is_pressed() {
                match key.get_keycode() {
                    Key::B => self.animate("blink", true),
                    Key::F => self.animate("flash_eyes", true),
                    Key::S => self.animate("shift_eyes", true),
                    Key::Q => self.animate("default", true),
                    _ => (),
                }
            }
        }
    }
}

#[godot_api]
impl Alligator {
    fn upper_jaw(&self) -> Gd<AnimatedSprite2D> {
        self.base()
            .get_node_as::<AnimatedSprite2D>("lower_jaw/upper_jaw")
    }

    fn animate(&self, anim: &str, force: bool) {
        let mut sprite = self.upper_jaw();
        if !sprite.is_playing() || force {
            sprite.play_ex().name(anim).done();
        }
    }

    #[func]
    fn on_idle_timeout(&self) {
        let animation = if randf() > 0.5 { "blink" } else { "shift_eyes" };
        self.animate(&animation, false);
        self.start_idle_timer();
    }

    fn start_idle_timer(&self) {
        let wait_time = randf_range(3.0, 7.0);
        let mut timer = self.idle_timer();
        timer.start_ex().time_sec(wait_time).done();
    }

    fn idle_timer(&self) -> Gd<Timer> {
        self.base().get_node_as::<Timer>("IdleTimer")
    }
}
