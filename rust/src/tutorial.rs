use godot::classes::{Input, Label};
use godot::prelude::*;

use crate::button_hint::ButtonHint;
use crate::player::{Player, PlayerInfo};

/// Stages of the tutorial, using numbers to help keep them straight. Each step
/// has a corresponding member variable that specifies how long after the
/// previous step it should start.
enum NextStep {
    OneStartJump,
    TwoReleaseJump,
    ThreeShowArrow,
    FourReset,
    FiveStartJump,
    SixReleaseJump,
    SevenShowArrow,
    EightReload,
}

/// Tutorial that demonstrates how jumping works. The script will hold the jump
/// key for different lengths of time for different strength jumps.
#[derive(GodotClass)]
#[class(base=Node2D)]
struct Tutorial {
    #[export]
    one_start_jump_ms: f32,
    #[export]
    two_release_jump_ms: f32,
    #[export]
    three_show_arrow_ms: f32,
    #[export]
    four_reset_ms: f32,
    #[export]
    five_start_jump_ms: f32,
    #[export]
    six_release_jump_ms: f32,
    #[export]
    seven_show_arrow_ms: f32,
    #[export]
    eight_reload_ms: f32,
    curr_time_ms: f64,
    next_step_time_ms: f64,
    next_step: NextStep,
    player_start_info: Option<PlayerInfo>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Tutorial {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            one_start_jump_ms: 500.0,
            two_release_jump_ms: 200.0,
            three_show_arrow_ms: 1200.0,
            four_reset_ms: 1000.0,
            five_start_jump_ms: 500.0,
            six_release_jump_ms: 600.0,
            seven_show_arrow_ms: 2000.0,
            eight_reload_ms: 1000.0,
            curr_time_ms: 0.0,
            next_step_time_ms: f64::default(),
            next_step: NextStep::OneStartJump,
            player_start_info: None,
            base,
        }
    }

    fn ready(&mut self) {
        self.player_start_info = Some(self.player().bind().get_player_info());
        self.next_step_time_ms = self.one_start_jump_ms as f64;
    }

    fn physics_process(&mut self, delta: f64) {
        self.curr_time_ms += delta * 1000.0;
        if self.curr_time_ms < self.next_step_time_ms {
            return;
        }
        match self.next_step {
            NextStep::OneStartJump => {
                self.set_pressed(true);
                self.next_step = NextStep::TwoReleaseJump;
                self.next_step_time_ms += self.two_release_jump_ms as f64;
            }
            NextStep::TwoReleaseJump => {
                self.set_pressed(false);
                self.next_step = NextStep::ThreeShowArrow;
                self.next_step_time_ms += self.three_show_arrow_ms as f64;
            }
            NextStep::ThreeShowArrow => {
                self.short_press_label().show();
                self.next_step = NextStep::FourReset;
                self.next_step_time_ms += self.four_reset_ms as f64;
            }
            NextStep::FourReset => {
                self.short_press_label().hide();
                self.reset_player();
                self.next_step = NextStep::FiveStartJump;
                self.next_step_time_ms += self.five_start_jump_ms as f64;
            }
            NextStep::FiveStartJump => {
                self.set_pressed(true);
                self.next_step = NextStep::SixReleaseJump;
                self.next_step_time_ms += self.six_release_jump_ms as f64;
            }
            NextStep::SixReleaseJump => {
                self.set_pressed(false);
                self.next_step = NextStep::SevenShowArrow;
                self.next_step_time_ms += self.seven_show_arrow_ms as f64;
            }
            NextStep::SevenShowArrow => {
                self.long_press_label().show();
                self.next_step = NextStep::EightReload;
                self.next_step_time_ms += self.eight_reload_ms as f64;
            }
            NextStep::EightReload => {
                self.long_press_label().hide();
                self.next_step = NextStep::OneStartJump;
                self.next_step_time_ms = self.one_start_jump_ms as f64;
                self.reset_player();
                self.curr_time_ms = 0.0;
            }
        }
    }
}

impl Tutorial {
    fn set_pressed(&mut self, pressed: bool) {
        if pressed {
            Input::singleton().action_press("jump");
        } else {
            Input::singleton().action_release("jump");
        }
        self.button_hint().bind_mut().set_pressed(pressed);
    }

    fn button_hint(&self) -> Gd<ButtonHint> {
        self.base().get_node_as::<ButtonHint>("HUD/ButtonHint")
    }

    fn short_press_label(&self) -> Gd<Label> {
        self.base().get_node_as::<Label>("ShortPress")
    }

    fn long_press_label(&self) -> Gd<Label> {
        self.base().get_node_as::<Label>("LongPress")
    }

    fn player(&self) -> Gd<Player> {
        self.base().get_node_as::<Player>("Player")
    }

    fn reset_player(&self) {
        self.player().bind_mut().set_player_info(
            &self
                .player_start_info
                .expect("`ready` should have instantiated `playerStartInfo`"),
        );
    }
}
