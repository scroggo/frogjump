use std::fmt::Display;

use godot::classes::{AnimatedSprite2D, Area2D, Node2D, Timer};
use godot::global::{absf, clampf, maxf, randf, randf_range};
use godot::prelude::*;

#[derive(PartialEq, Clone)]
enum State {
    Idle,
    // If the player is in the focus area 2d, track them.
    Focus { player: Gd<Node2D> },
    OpeningJaw { player: Gd<Node2D> },
    ClosingJaw { player: Gd<Node2D> },
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            State::Idle => "Idle",
            State::Focus { player: _ } => "Focus",
            State::OpeningJaw { player: _ } => "Opening Jaw",
            State::ClosingJaw { player: _ } => "Closing Jaw",
        };
        write!(f, "{}", s)
    }
}

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct Alligator {
    // TODO: What happens to this if we try to free the player?
    state: State,
    #[export]
    jaw_close_speed: f32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Alligator {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            state: State::Idle,
            jaw_close_speed: 512.0,
            base,
        }
    }

    fn ready(&mut self) {
        let idle_timeout = self.base().callable("on_idle_timeout");
        self.base()
            .get_node_as::<Timer>("IdleTimer")
            .connect("timeout", &idle_timeout);
        self.start_idle_timer();

        let mut focus_area = self.focus_area2d();
        let player_entered = self.base().callable("on_player_entered_focus_area");
        focus_area.connect("body_entered", &player_entered);

        let player_exited = self.base().callable("on_player_exited_focus_area");
        focus_area.connect("body_exited", &player_exited);

        let mut eat_area = self.eat_area2d();
        let player_entered_eat_area = self.base().callable("on_player_entered_eat_area");
        eat_area.connect("body_entered", &player_entered_eat_area);

        let player_exited_eat_area = self.base().callable("on_player_exited_eat_area");
        eat_area.connect("body_exited", &player_exited_eat_area);
    }

    fn physics_process(&mut self, delta: f64) {
        match self.state.clone() {
            State::Focus { player } => {
                self.face_player(player);
            }
            State::OpeningJaw { player } => {
                // Calculate how wide the jaw should be open based on distance
                // to the player. Based on trial and error, I want the jaw to
                // be open 60 degrees when the player is at distance 0, and
                // closed (0 degrees) when the player is at distance 38.
                let d = absf((player.get_position().x - self.base().get_position().x).into());
                let rotation = 60.0 - clampf(d, 0.0, 60.0);
                self.upper_jaw().set_rotation_degrees(rotation as f32);

                // I am tempted to call `face_player`, but this could result in
                // flipping the alligator at the last moment. I only think I
                // want to face the player because my test scene lets me warp
                // the player.
                //self.face_player(player);

                // Clamp down when the player gets close.
                if d < 5.0 {
                    self.state = State::ClosingJaw { player };
                    godot_print!("Eat the player!");
                }
            }
            State::ClosingJaw { player } => {
                let mut jaw = self.upper_jaw();
                let curr_rotation = jaw.get_rotation_degrees();
                let new_rotation = maxf(
                    0.0,
                    (curr_rotation - delta as f32 * self.jaw_close_speed).into(),
                );
                jaw.set_rotation_degrees(new_rotation as f32);
                // TODO: Remove player.
                if new_rotation == 0.0 {
                    // TODO: Gloat!
                    self.state = State::Idle;
                }
            }
            _ => (),
        }
    }
}

#[godot_api]
impl Alligator {
    fn upper_jaw(&self) -> Gd<AnimatedSprite2D> {
        self.base()
            .get_node_as::<AnimatedSprite2D>("lower_jaw/upper_jaw")
    }

    pub fn animate(&self, anim: &str, force: bool) {
        let mut sprite = self.upper_jaw();
        if !sprite.is_playing() || force {
            sprite.play_ex().name(anim).done();
        }
    }

    fn face_player(&mut self, player: Gd<Node2D>) {
        match player.get_position().x - self.base().get_position().x {
            0.0 => (),
            d if d < 0.0 => {
                self.base_mut().set_scale(Vector2 { x: 1.0, y: 1.0 });
            }
            d if d > 0.0 => self.base_mut().set_scale(Vector2 { x: -1.0, y: 1.0 }),
            // The positions should always subtract; i.e. in practice we do not
            // need to handle infinities/NaN.
            _ => (),
        }
    }

    #[func]
    fn on_idle_timeout(&self) {
        if self.state == State::Idle {
            let animation = if randf() > 0.5 { "blink" } else { "shift_eyes" };
            self.animate(&animation, false);
        }
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

    fn focus_area2d(&self) -> Gd<Area2D> {
        self.base().get_node_as::<Area2D>("FocusArea2D")
    }

    #[func]
    fn on_player_entered_focus_area(&mut self, body: Gd<Node2D>) {
        // Assume that `body` is the player, since the mask is set to only scan
        // for the player's layer.
        self.state = State::Focus {
            player: body.clone(),
        };
        self.animate("default", true);
    }

    #[func]
    fn on_player_exited_focus_area(&mut self, _body: Gd<Node2D>) {
        self.state = State::Idle;
        godot_print!("Player exited focus area! Now in state {}", self.state);
    }

    fn eat_area2d(&self) -> Gd<Area2D> {
        self.base().get_node_as::<Area2D>("EatArea2D")
    }

    #[func]
    fn on_player_entered_eat_area(&mut self, body: Gd<Node2D>) {
        self.state = State::OpeningJaw {
            player: body.clone(),
        };
        self.animate("flash_eyes", true);
    }

    #[func]
    fn on_player_exited_eat_area(&mut self, _body: Gd<Node2D>) {
        // If the player warps outside the focus area from the eat area, it
        // seems that the order of this signal and `on_player_exited_focus_area`
        // is unspecified. (I've seen both orders.) So treat the two as though
        // they may happen in either order. Note that this is primarily relevant
        // in the `test_alligator.tscn`, when the (fake) player can actually
        // warp. (Maybe this is totally irrelevant during gameplay?)
        match self.state.clone() {
            State::OpeningJaw { player } => {
                self.state = if self.focus_area2d().overlaps_body(&player) {
                    State::Focus { player }
                } else {
                    State::Idle
                };
                self.animate("default", true);
            }
            _ => (),
        }
        godot_print!("Player exited EAT area! Now in state: {}", self.state);
    }
}
