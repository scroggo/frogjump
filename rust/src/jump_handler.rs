use crate::jump_meter::JumpMeter;
use godot::classes::Time;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct JumpHandler {
    // If `None`, jump is not pressed.
    // If `Some`, the start of holding jump.
    start_of_jump_press_ms: Option<u64>,

    // Maximum time to make jump more powerful.
    #[export]
    max_time_ms: i32,
    base: Base<Node>,
}

#[godot_api]
impl INode for JumpHandler {
    fn init(base: Base<Node>) -> Self {
        Self {
            start_of_jump_press_ms: None,
            max_time_ms: 1000,
            base,
        }
    }

    fn ready(&mut self) {
        self.jump_meter().hide();
    }
}

impl JumpHandler {
    // Player jumps based on how long the jump button was held before releasing.
    // Check input. If the player should jump, return `Some<float>`, where
    // `float` is between `0` and `1` and `1` is a max strength jump.
    pub fn handle_input(&mut self) -> Option<f32> {
        let input = Input::singleton();
        if self.start_of_jump_press_ms.is_none() {
            if input.is_action_pressed("jump") {
                self.start_of_jump_press_ms = Some(Time::singleton().get_ticks_msec());
                let mut jump_meter = self.jump_meter();
                jump_meter.bind_mut().set_ratio(0.0);
                jump_meter.show();
            }
            return None;
        }
        let duration_ms = Time::singleton().get_ticks_msec() - self.start_of_jump_press_ms.unwrap();
        let strength = match duration_ms {
            duration if duration >= self.max_time_ms as u64 => 1.0,
            duration => duration as f32 / self.max_time_ms as f32,
        };
        if input.is_action_pressed("jump") {
            // Still holding jump.
            self.jump_meter().bind_mut().set_ratio(strength);
            return None;
        }
        // Released jump.
        self.start_of_jump_press_ms = None;
        self.jump_meter().hide();
        Some(strength)
    }

    fn jump_meter(&self) -> Gd<JumpMeter> {
        self.base().get_node_as::<JumpMeter>("../JumpMeter")
    }
}
