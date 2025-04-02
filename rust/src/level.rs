use crate::player::Player;
use crate::player::PlayerInfo;
use godot::classes::{
    ITileMapLayer, InputEvent, InputEventKey, InputEventScreenTouch, TileMapLayer,
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
            base,
        }
    }

    fn ready(&mut self) {
        let on_player_eaten = self.base().callable("on_player_eaten");
        self.base_mut().get_tree().unwrap().call_group(
            "predators",
            "connect",
            &["player_eaten".to_variant(), on_player_eaten.to_variant()],
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
            if key_event.get_keycode() == Key::SPACE
                && (self.player().is_none() || self.spawn_many_frogs)
            {
                self.respawn();
            }
            if self.is_test_level {
                match key_event.get_keycode() {
                    Key::ESCAPE => {
                        self.base().get_tree().unwrap().reload_current_scene();
                    }
                    Key::ENTER => {
                        if let Some(scene) = &self.next_level {
                            self.base()
                                .get_tree()
                                .unwrap()
                                .change_scene_to_packed(scene);
                        }
                    }
                    _ => (),
                }
            }
            return;
        }
        if let Some(touch_event) = event.try_cast::<InputEventScreenTouch>().ok() {
            if touch_event.is_pressed() && (self.player().is_none() || self.spawn_many_frogs) {
                self.respawn();
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
            parent.remove_child(player.clone());
            player.queue_free();
        } else {
            godot_error!("player_eaten signal called on node not in tree?");
        }
    }

    fn player(&self) -> Option<Gd<Player>> {
        self.base().try_get_node_as::<Player>("Player")
    }

    fn respawn(&mut self) {
        let scene = load::<PackedScene>("res://player.tscn");
        let mut player = scene.instantiate().unwrap().cast::<Player>();
        player.bind_mut().set_player_info(&self.player_respawn_info);
        self.base_mut().add_child(player);
    }
}
