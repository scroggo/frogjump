use crate::player::Player;
use crate::player::PlayerInfo;
use godot::classes::{Camera2D, ITileMapLayer, InputEvent, TileMapLayer, Timer};
use godot::prelude::*;

#[derive(PartialEq)]
enum State {
    Playing,
    JumpToRespawn,
    Won,
    BonusFound,
}

/// Code for playing a level.
#[derive(GodotClass)]
#[class(base=TileMapLayer)]
pub struct Level {
    player_respawn_info: Option<PlayerInfo>,
    state: State,
    base: Base<TileMapLayer>,
}

#[godot_api]
impl ITileMapLayer for Level {
    fn init(base: Base<TileMapLayer>) -> Self {
        Self {
            player_respawn_info: None,
            state: State::Playing,
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

        let on_bonus_found = self.base().callable("on_bonus_found");
        scene_tree.call_group(
            "bonus_prey",
            "connect",
            &["eaten".to_variant(), on_bonus_found.to_variant()],
        );

        if let Some(player) = self.player() {
            self.player_respawn_info = Some(player.bind().get_player_info());
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("jump") {
            if self.state == State::JumpToRespawn {
                self.respawn();
                self.state = State::Playing;
            }
        }
    }
}

#[godot_api]
impl Level {
    #[signal]
    pub fn complete_level();
    #[signal]
    pub fn find_bonus();

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
        if let Some(mut scene_tree) = self.base().get_tree() {
            // Support multiple players with different names.
            let players_remaining = scene_tree.get_nodes_in_group("player").len();
            if players_remaining == 0 && self.state == State::Playing {
                self.state = State::JumpToRespawn;
                self.respawn_hint_delay_timer()
                    .start_ex()
                    .time_sec(1.0)
                    .done();
            }
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
                self.disable_jumping();
                self.signals().complete_level().emit();
            }
        }
    }

    #[func]
    fn on_bonus_found(&mut self) {
        self.state = State::BonusFound;
        self.disable_jumping();
        self.signals().find_bonus().emit();
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
        if let Some(mut respawn_hint) = self.respawn_hint(false) {
            self.base_mut().remove_child(&respawn_hint);
            respawn_hint.queue_free();
        }
    }

    fn respawn_hint_delay_timer(&mut self) -> Gd<Timer> {
        let name = "RespawnHintDelayTimer";
        if let Some(timer) = self.base().try_get_node_as::<Timer>(name) {
            return timer;
        }
        let mut timer = Timer::new_alloc();
        timer.set_name(name);
        timer.set_one_shot(true);
        self.base_mut().add_child(&timer);
        let gd = Gd::from_instance_id(self.base().instance_id());
        timer
            .signals()
            .timeout()
            .connect_obj(&gd, Self::on_respawn_hint_timeout);
        timer
    }

    #[func]
    fn on_respawn_hint_timeout(&mut self) {
        // Ensure that the player has not already respawned.
        if self.state == State::JumpToRespawn {
            if let Some(hint) = self.respawn_hint(true) {
                self.base_mut().add_child(&hint);
            }
        }
    }

    fn respawn_hint(&self, create: bool) -> Option<Gd<Node>> {
        let name = "RespawnHint";
        if let Some(hint) = self.base().try_get_node_as::<Node>(name) {
            return Some(hint);
        }
        if !create {
            return None;
        }
        let packed_scene = load::<PackedScene>("res://ui/respawn_hint.tscn");
        let mut hint = packed_scene.instantiate_as::<Node>();
        hint.set_name(name);
        Some(hint)
    }

    fn disable_jumping(&self) {
        let mut scene_tree = self.base().get_tree().unwrap();
        scene_tree.call_group("player", "disable_jumping", &[]);
    }
}
