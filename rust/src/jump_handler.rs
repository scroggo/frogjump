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
}

impl JumpHandler {
    // Player jumps based on how long the jump button was held before releasing.
    // Check input. If the player should jump, return `Some<float>`, where
    // `float` is between `0` and `1` and `1` is a max strength jump.
    pub fn handle_input(&mut self) -> Option<f32> {
        let input = Input::singleton();
        if self.start_of_jump_press_ms.is_none() {
            if input.is_action_just_pressed("jump") {
                self.start_of_jump_press_ms = Some(Time::singleton().get_ticks_msec());
                godot_print!("Prepare to jump...");
            }
            return None;
        }
        if input.is_action_pressed("jump") {
            // Still holding jump.
            return None;
        }
        // Released jump.
        let duration_ms =
            Time::singleton().get_ticks_msec() - self.start_of_jump_press_ms.take().unwrap();
        if duration_ms >= self.max_time_ms as u64 {
            godot_print!("Full strength jump!");
            return Some(1.0);
        }
        let strength = duration_ms as f32 / self.max_time_ms as f32;
        godot_print!("Jump strength: {strength}");
        Some(strength)
    }
}
