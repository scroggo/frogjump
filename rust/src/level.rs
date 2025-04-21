use crate::message_screen::MessageScreen;
use crate::player::Player;
use crate::player::PlayerInfo;
use godot::classes::{Camera2D, ITileMapLayer, InputEvent, TileMapLayer};
use godot::prelude::*;

#[derive(PartialEq)]
enum State {
    Playing,
    Won,
    BonusFound,
}

/// Code for playing a level.
#[derive(GodotClass)]
#[class(base=TileMapLayer)]
struct Level {
    player_respawn_info: Option<PlayerInfo>,
    /// Level to switch to after completing this one.
    #[export]
    next_level: Option<Gd<PackedScene>>,
    /// Level to switch to upon triggering the bonus condition.
    #[export]
    bonus_level: Option<Gd<PackedScene>>,
    state: State,
    base: Base<TileMapLayer>,
}

#[godot_api]
impl ITileMapLayer for Level {
    fn init(base: Base<TileMapLayer>) -> Self {
        Self {
            player_respawn_info: None,
            next_level: None,
            bonus_level: None,
            state: State::Playing,
            base,
        }
    }

    fn ready(&mut self) {
        let mut scene_tree = self.base_mut().get_tree().unwrap();
        scene_tree.call_group("player", "consume_input", &[true.to_variant()]);

        let on_player_eaten = self.base().callable("on_player_eaten");
        scene_tree.call_group(
            "predators",
            "connect",
            &["player_eaten".to_variant(), on_player_eaten.to_variant()],
        );

        let on_prey_eaten = self.base().callable("on_prey_eaten");
        scene_tree.call_group(
            "prey",
            "connect",
            &["eaten".to_variant(), on_prey_eaten.to_variant()],
        );

        if self.bonus_level.is_some() {
            let on_bonus_found = self.base().callable("on_bonus_found");
            scene_tree.call_group(
                "bonus_prey",
                "connect",
                &["eaten".to_variant(), on_bonus_found.to_variant()],
            );
        }

        if let Some(player) = self.player() {
            self.player_respawn_info = Some(player.bind().get_player_info());
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("jump") {
            if self.state == State::Playing {
                if self.player().is_none() {
                    self.respawn();
                }
            }
        } else if event.is_action_pressed("RELOAD") {
            self.base().get_tree().unwrap().reload_current_scene();
        } else if event.is_action_pressed("NEXT") {
            self.load_next();
        }
    }
}

#[godot_api]
impl Level {
    #[func]
    fn on_player_eaten(&mut self, mut player: Gd<Node2D>) {
        godot_print!("on_player_eaten! eating {}", player.get_name());
        if let Some(mut parent) = player.get_parent() {
            if let Some(mut camera) = player.try_get_node_as::<Camera2D>("Camera2D") {
                // Reparent the camera so it can stay in place when the player
                // is removed.
                player.remove_child(&camera);
                camera.set_position(player.get_position());
                self.base_mut().add_child(&camera);
            }
            parent.remove_child(&player);
            player.queue_free();
        } else {
            godot_error!("player_eaten signal called on node not in tree?");
        }
    }

    #[func]
    fn on_prey_eaten(&mut self) {
        if let Some(mut scene_tree) = self.base().get_tree() {
            // When the last prey is eaten, it is queued for removal, but the
            // signal should call this method before the prey is removed.
            let prey_remaining = scene_tree.get_nodes_in_group("prey").len();
            if prey_remaining <= 1 {
                self.state = State::Won;
                let scene_name = if self.next_level.is_some() {
                    "res://messages/finish_level.tscn"
                } else {
                    "res://messages/finish_final_level.tscn"
                };
                let packed_scene = load::<PackedScene>(scene_name);
                let message_screen = packed_scene.instantiate_as::<MessageScreen>();
                self.show_message_screen(message_screen);
            }
        }
    }

    #[func]
    fn on_bonus_found(&mut self) {
        self.state = State::BonusFound;
        let scene = load::<PackedScene>("res://messages/bonus.tscn");
        let bonus_message = scene.instantiate_as::<MessageScreen>();
        self.show_message_screen(bonus_message);
    }

    #[func]
    fn on_message_screen_jump_pressed(&self) {
        match self.state {
            State::Playing => (),
            State::Won => {
                self.load_next();
            }
            State::BonusFound => {
                if let Some(bonus_level) = &self.bonus_level {
                    self.base()
                        .get_tree()
                        .unwrap()
                        .change_scene_to_packed(bonus_level);
                } else {
                    // This should not happen. We should only reach this state
                    // if there is a bonus level to go to.
                    godot_error!("Missing bonus level!");
                }
            }
        }
    }

    fn player(&self) -> Option<Gd<Player>> {
        self.base().try_get_node_as::<Player>("Player")
    }

    fn respawn(&mut self) {
        if let Some(respawn_info) = &self.player_respawn_info {
            let scene = load::<PackedScene>("res://player.tscn");
            let mut player = scene.instantiate().unwrap().cast::<Player>();
            player.bind_mut().set_player_info(respawn_info);

            // When the player dies, we reparent the camera to the level. Restore it
            // on the new player.
            if let Some(mut camera) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
                self.base_mut().remove_child(&camera);
                camera.set_position(Vector2::ZERO);
                player.add_child(&camera);
            }
            self.base_mut().add_child(&player);
        }
    }

    fn load_next(&self) {
        let mut scene_tree = self.base().get_tree().unwrap();
        if let Some(scene) = &self.next_level {
            scene_tree.change_scene_to_packed(scene);
        } else {
            scene_tree.change_scene_to_file("res://levels/tutorial.tscn");
        }
    }

    fn show_message_screen(&mut self, mut message_screen: Gd<MessageScreen>) {
        let cb = self.base().callable("on_message_screen_jump_pressed");
        message_screen.connect("jump_pressed", &cb);
        let mut scene_tree = self.base().get_tree().unwrap();
        scene_tree.call_group("player", "disable_jumping", &[]);
        scene_tree.call_group("player", "consume_input", &[false.to_variant()]);
        self.base_mut().add_child(&message_screen);
    }
}
