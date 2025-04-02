use crate::player::Player;
use crate::player::PlayerInfo;
use godot::classes::{
    ITileMapLayer, InputEvent, InputEventKey, InputEventScreenTouch, Label, TileMapLayer,
};
use godot::global::Key;
use godot::prelude::*;

/// Code for playing a level.
#[derive(GodotClass)]
#[class(base=TileMapLayer)]
struct Level {
    player_respawn_info: PlayerInfo,

    /// Spawn a frog each time "jump" is pressed. Setting this to true recreates
    /// some accidental behavior that was fun.
    #[export]
    spawn_many_frogs: bool,

    /// Enable some testing-only features like ESCAPE to restart and ENTER to
    /// start the next level.
    #[export]
    is_test_level: bool,

    /// Level to switch to after completing this one.
    #[export]
    next_level: Option<Gd<PackedScene>>,
    won: bool,
    base: Base<TileMapLayer>,
}

#[godot_api]
impl ITileMapLayer for Level {
    fn init(base: Base<TileMapLayer>) -> Self {
        Self {
            player_respawn_info: PlayerInfo::default(),
            spawn_many_frogs: false,
            is_test_level: false,
            next_level: None,
            won: false,
            base,
        }
    }

    fn ready(&mut self) {
        let mut scene_tree = self.base_mut().get_tree().unwrap();
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

        if let Some(player) = self.player() {
            self.player_respawn_info = player.bind().get_player_info();
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Some(key_event) = event.clone().try_cast::<InputEventKey>().ok() {
            if key_event.is_echo() || !key_event.is_pressed() {
                return;
            }
            if key_event.get_keycode() == Key::SPACE {
                if self.won {
                    self.load_next_if_any();
                    return;
                }
                if self.player().is_none() || self.spawn_many_frogs {
                    self.respawn();
                }
            }
            if self.is_test_level {
                match key_event.get_keycode() {
                    Key::ESCAPE => {
                        self.base().get_tree().unwrap().reload_current_scene();
                    }
                    Key::ENTER => {
                        self.load_next_if_any();
                    }
                    _ => (),
                }
            }
            return;
        }
        if let Some(touch_event) = event.try_cast::<InputEventScreenTouch>().ok() {
            if touch_event.is_pressed() {
                if self.won {
                    self.load_next_if_any();
                    return;
                }
                if self.player().is_none() || self.spawn_many_frogs {
                    self.respawn();
                }
            }
            return;
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
                player.remove_child(camera.clone());
                camera.set_position(player.get_position());
                self.base_mut().add_child(camera);
            }
            parent.remove_child(player.clone());
            player.queue_free();
        } else {
            godot_error!("player_eaten signal called on node not in tree?");
        }
    }

    #[func]
    fn on_prey_eaten(&mut self) {
        let base_mut = self.base_mut();
        if let Some(mut scene_tree) = base_mut.get_tree() {
            // When the last prey is eaten, it is queued for removal, but the
            // signal should call this method before the prey is removed.
            let prey_remaining = scene_tree.get_nodes_in_group("prey").len();
            if prey_remaining <= 1 {
                if let Some(mut win_message) = base_mut.try_get_node_as::<Label>("WinMessage") {
                    // Show the "WinMessage" in front of the camera, centered.
                    if let Some(camera) = win_message
                        .get_viewport()
                        .and_then(|viewport| viewport.get_camera_2d())
                    {
                        let position =
                            camera.get_screen_center_position() - win_message.get_size() / 2.0;
                        win_message.set_global_position(position);
                    }
                    win_message.show();
                    drop(base_mut);
                    self.won = true;
                }
            }
        }
    }

    fn player(&self) -> Option<Gd<Player>> {
        self.base().try_get_node_as::<Player>("Player")
    }

    fn respawn(&mut self) {
        let scene = load::<PackedScene>("res://player.tscn");
        let mut player = scene.instantiate().unwrap().cast::<Player>();
        player.bind_mut().set_player_info(&self.player_respawn_info);

        // When the player dies, we reparent the camera to the level. Restore it
        // on the new player.
        if let Some(mut camera) = self.base().try_get_node_as::<Camera2D>("Camera2D") {
            self.base_mut().remove_child(camera.clone());
            camera.set_position(Vector2::ZERO);
            player.add_child(camera);
        }
        self.base_mut().add_child(player);
    }

    fn load_next_if_any(&self) {
        if let Some(scene) = &self.next_level {
            self.base()
                .get_tree()
                .unwrap()
                .change_scene_to_packed(scene);
        }
    }
}
