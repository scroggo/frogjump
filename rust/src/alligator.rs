use godot::classes::{AnimatedSprite2D, Area2D, InputEvent, InputEventKey, Node2D, Timer};
use godot::global::{randf, randf_range, Key};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct Alligator {
    // If the player is in the focus area 2d, track them.
    // TODO: What happens to this if we try to free the player?
    focused_player: Option<Gd<Node2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Alligator {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            focused_player: None,
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
    }

    fn physics_process(&mut self, _delta: f64) {
        if self.focused_player.is_some() {
            match self.focused_player.as_ref().unwrap().get_position().x
                - self.base().get_position().x
            {
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
        if self.focused_player.is_none() {
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
        if self.focused_player.is_some() {
            godot_error!("A second player entered?");
            return;
        }
        self.focused_player = Some(body.clone());
        self.animate("default", true);
    }

    #[func]
    fn on_player_exited_focus_area(&mut self, body: Gd<Node2D>) {
        if self.focused_player.is_none() {
            godot_error!("Body exited without already being there?");
            return;
        }
        if body.instance_id() != self.focused_player.as_ref().unwrap().instance_id() {
            godot_error!("Are there multiple players?");
            return;
        }
        godot_print!("Player exited focus area!");
        self.focused_player = None;
    }

    fn eat_area2d(&self) -> Gd<Area2D> {
        self.base().get_node_as::<Area2D>("EatArea2D")
    }

    #[func]
    fn on_player_entered_eat_area(&mut self, body: Gd<Node2D>) {
        self.animate("flash_eyes", true);
        self.upper_jaw().set_rotation_degrees(60.0);
    }

    /* Hmm, do we need this? Or will the player just get eaten?
    #[func]
    fn on_player_exited_eat_area(&mut self, body: Gd<Node2D>) {
    }
    */
}
