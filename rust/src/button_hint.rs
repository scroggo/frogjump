use godot::classes::{AnimatedSprite2D, Control, IControl, Os};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct ButtonHint {
    on_mobile: bool,
    base: Base<Control>,
}

#[godot_api]
impl IControl for ButtonHint {
    fn init(base: Base<Control>) -> Self {
        Self {
            on_mobile: is_mobile(),
            base,
        }
    }

    fn ready(&mut self) {
        if self.on_mobile {
            self.button().hide();
            self.tap().show();
        } else {
            self.button().show();
            self.tap().hide();
        }
    }
}

impl ButtonHint {
    pub fn set_pressed(&self, pressed: bool) {
        let custom_speed = if pressed { 1.0 } else { -1.0 };
        let mut sprite = if self.on_mobile {
            self.tap()
        } else {
            self.button()
        };
        sprite
            .play_ex()
            .name("press")
            .custom_speed(custom_speed)
            .from_end(!pressed)
            .done();
    }

    fn button(&self) -> Gd<AnimatedSprite2D> {
        self.base().get_node_as::<AnimatedSprite2D>("Button")
    }

    fn tap(&self) -> Gd<AnimatedSprite2D> {
        self.base().get_node_as::<AnimatedSprite2D>("Tap")
    }
}

fn is_mobile() -> bool {
    let os = Os::singleton();
    os.has_feature("web_android") || os.has_feature("web_ios")
}
