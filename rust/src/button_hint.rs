use godot::classes::{AnimatedSprite2D, Control, IControl, Os, Timer};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct ButtonHint {
    /// Whether to autoplay the animation.
    #[export]
    autoplay: bool,
    /// When autoplaying, how long to wait between changing.
    #[export]
    delay_sec: f32,
    pressed: bool,
    on_mobile: bool,
    base: Base<Control>,
}

#[godot_api]
impl IControl for ButtonHint {
    fn init(base: Base<Control>) -> Self {
        Self {
            autoplay: false,
            delay_sec: 1.0,
            pressed: false,
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
        if self.autoplay {
            if self.base().is_visible() {
                self.animate();
            } else {
                let callable = self.base().callable("animate");
                self.base_mut().connect("visibility_changed", &callable);
            }
        }
    }
}

#[godot_api]
impl ButtonHint {
    pub fn set_pressed(&mut self, pressed: bool) {
        if self.autoplay {
            godot_error!("ButtonHint::set_pressed should not be called when autoplaying!");
        }
        self.set_pressed_internal(pressed);
    }

    fn set_pressed_internal(&mut self, pressed: bool) {
        if self.pressed == pressed {
            return;
        }
        self.pressed = pressed;
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

    #[func]
    fn animate(&mut self) {
        let new_pressed_state = !self.pressed;
        self.set_pressed_internal(new_pressed_state);

        let name = "Timer";
        let mut timer;
        if let Some(t) = self.base().try_get_node_as::<Timer>(name) {
            timer = t;
        } else {
            timer = Timer::new_alloc();
            timer.set_name(name);
            self.base_mut().add_child(&timer);
            let callable = self.base().callable("animate");
            timer.connect("timeout", &callable);
        }

        timer.start_ex().time_sec(self.delay_sec.into()).done();
    }
}

fn is_mobile() -> bool {
    let os = Os::singleton();
    os.has_feature("web_android") || os.has_feature("web_ios")
}
