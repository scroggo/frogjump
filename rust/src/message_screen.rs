use godot::classes::{CanvasLayer, Control, ICanvasLayer, InputEvent, Timer};
use godot::prelude::*;

/// Message screen to show when finishing a level
#[derive(GodotClass)]
#[class(base=CanvasLayer)]
pub struct MessageScreen {
    /// How long in seconds to delay before allowing bypassing the screen.
    #[export]
    delay: f32,
    ignore_jump: bool,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for MessageScreen {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            delay: 1.0,
            ignore_jump: true,
            base,
        }
    }

    fn ready(&mut self) {
        // Spawn a timer in code to simplify the various scenes of this type.
        let mut timer = Timer::new_alloc();
        self.base_mut().add_child(&timer);
        let timeout = self.base().callable("on_timeout");
        timer.connect("timeout", &timeout);
        timer.start_ex().time_sec(self.delay as f64).done();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if !self.ignore_jump && event.is_action_pressed("jump") {
            self.base_mut().emit_signal("jump_pressed", &[]);
        }
    }
}

#[godot_api]
impl MessageScreen {
    #[signal]
    fn jump_pressed();

    fn jump_hint(&self) -> Option<Gd<Control>> {
        self.base().try_get_node_as::<Control>("JumpHint")
    }

    #[func]
    fn on_timeout(&mut self) {
        if let Some(mut jump_hint) = self.jump_hint() {
            jump_hint.show();
        }
        self.ignore_jump = false;
    }
}
