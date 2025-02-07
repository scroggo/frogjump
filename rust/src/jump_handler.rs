use crate::jump_meter::JumpMeter;
use godot::prelude::*;

pub trait JumpDetector {
    fn is_jump_pressed(&self) -> bool;
}

struct JumpKeyDetector;

impl JumpDetector for JumpKeyDetector {
    fn is_jump_pressed(&self) -> bool {
        Input::singleton().is_action_pressed("jump")
    }
}

impl JumpKeyDetector {
    fn new() -> Self {
        Self
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct JumpHandler {
    // If `None`, jump is not pressed.
    // If `Some`, how long jump has been held.
    length_of_jump_press_ms: Option<f32>,

    // Maximum time to make jump more powerful.
    #[export]
    max_time_ms: f32,
    // Can be used to ensure hitting a particular number for testing.
    #[export]
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
