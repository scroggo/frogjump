use crate::jump_meter::JumpMeter;
use godot::prelude::*;

pub trait JumpDetector {
    fn is_jump_pressed(&mut self) -> bool;

    // Call early to determine whether jump is already held when gameplay starts
    // and prevent filling the jump meter until the next press.
    fn check_for_early_jump(&mut self) {}
}

struct JumpKeyDetector {
    wait_for_next_press: bool,
}

impl JumpDetector for JumpKeyDetector {
    fn is_jump_pressed(&mut self) -> bool {
        let jump_pressed = Input::singleton().is_action_pressed("jump");
        if self.wait_for_next_press {
            if !jump_pressed {
                self.wait_for_next_press = false;
            }
            return false;
        }
        jump_pressed
    }

    fn check_for_early_jump(&mut self) {
        if self.is_jump_pressed() {
            self.wait_for_next_press = true;
        }
    }
}

impl JumpKeyDetector {
    fn new() -> Self {
        Self {
            wait_for_next_press: false,
        }
    }
}

/// This struct handles converting input into whether to jump and the jump's
/// strength, if so.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct JumpHandler {
    /// If `None`, jump is not pressed.
    /// If `Some`, how long jump has been held.
    length_of_jump_press_ms: Option<f32>,

    /// How long it takes to max out the jump meter.
    #[export]
    max_time_ms: f32,
    /// Limit the jump strength to a ratio out of 1. If set to 0, the player
    /// cannot jump. At 0.5, the player's jump will max out at half strength.
    /// Can be useful to ensure hitting a particular strength for testing.
    #[export(range = (0.0, 1.0))]
    max_jump_strength_for_testing: f32,
    jump_detector: Box<dyn JumpDetector>,
    base: Base<Node>,
}

#[godot_api]
impl INode for JumpHandler {
    fn init(base: Base<Node>) -> Self {
        Self {
            length_of_jump_press_ms: None,
            max_time_ms: 400.0,
            max_jump_strength_for_testing: 1.0,
            jump_detector: Box::new(JumpKeyDetector::new()),
            base,
        }
    }

    fn ready(&mut self) {
        self.jump_meter().hide();
        self.jump_detector.check_for_early_jump();
    }
}

impl JumpHandler {
    /// Player jumps based on how long the jump button was held before releasing.
    /// Check input. If the player should jump, return `Some<float>`, where
    /// `float` is between `0` and `1` and `1` is a max strength jump.
    /// `delta`: number of seconds since the last frame update.
    pub fn handle_input(&mut self, delta: f64) -> Option<f32> {
        if self.length_of_jump_press_ms.is_none() {
            if self.jump_detector.is_jump_pressed() {
                self.length_of_jump_press_ms = Some(0.0);
                let mut jump_meter = self.jump_meter();
                jump_meter.bind_mut().set_ratio(0.0);
                jump_meter.show();
            }
            return None;
        }
        let prev_duration_ms = self.length_of_jump_press_ms.unwrap();
        self.length_of_jump_press_ms = Some(prev_duration_ms + (delta * 1000.0) as f32);
        let mut strength = match self.length_of_jump_press_ms.unwrap() {
            duration if duration >= self.max_time_ms => 1.0,
            duration => duration / self.max_time_ms,
        };
        if strength > self.max_jump_strength_for_testing {
            strength = self.max_jump_strength_for_testing;
        }
        if self.jump_detector.is_jump_pressed() {
            // Still holding jump.
            self.jump_meter().bind_mut().set_ratio(strength);
            return None;
        }
        // Released jump.
        godot_print!("Jump strength: {strength}");
        self.length_of_jump_press_ms = None;
        self.jump_meter().hide();
        Some(strength)
    }

    fn jump_meter(&self) -> Gd<JumpMeter> {
        self.base().get_node_as::<JumpMeter>("../JumpMeter")
    }

    pub fn replace_jump_detector(&mut self, detector: Box<dyn JumpDetector>) {
        self.jump_detector = detector;
    }
}
