use crate::level::Level;
use crate::message_screen::MessageScreen;
use crate::tutorial::Tutorial;
use godot::classes::{AudioStreamPlayer, InputEvent};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    /// List of levels and other scenes (e.g. title screen) to play, in order.
    #[export]
    scenes: Array<Gd<PackedScene>>,
    /// Zero-based index of the scene to start on. Useful for testing.
    #[export]
    scene_index: i32,
    play_bonus_next: bool,
    /// Bonus level to play when finding the bonus fly.
    #[export]
    bonus_level: Option<Gd<PackedScene>>,
    active_scene: Option<Gd<Node>>,
    // Packed version of current scene for reloading.
    active_scene_packed: Option<Gd<PackedScene>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for Main {
    fn init(base: Base<Node>) -> Self {
        Self {
            scenes: Array::<Gd<PackedScene>>::new(),
            scene_index: 0,
            play_bonus_next: false,
            bonus_level: None,
            active_scene: None,
            active_scene_packed: None,
            base,
        }
    }

    fn ready(&mut self) {
        self.load_scene();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("RELOAD") {
            if let Some(packed_scene) = self.active_scene_packed.clone() {
                self.load_packed_scene(packed_scene);
            }
        } else if event.is_action_pressed("NEXT") {
            self.load_next_scene();
        }
    }
}

#[godot_api]
impl Main {
    fn load_scene(&mut self) {
        if let Some(packed_scene) = self.scenes.get(self.scene_index as usize) {
            godot_print!("Loading scene: {}", self.scene_index);
            self.load_packed_scene(packed_scene);

            // Play background sounds.
            // FIXME: This should perhaps be generalized/editable from the
            // editor, since we already have two `Main` nodes (one for the
            // game and one for testing), but I am not sure how best to do so.
            if let Some(mut audio_stream_player) = self
                .base()
                .try_get_node_as::<AudioStreamPlayer>("AudioStreamPlayer")
            {
                // Start playing from the first playable level.
                if self.scene_index >= 2 {
                    if !audio_stream_player.is_playing() {
                        audio_stream_player.play();
                    }
                } else {
                    audio_stream_player.stop();
                }
            }
        } else {
            godot_error!("Failed to load scene {}", self.scene_index);
        }
    }

    fn load_packed_scene(&mut self, packed_scene: Gd<PackedScene>) {
        let node = packed_scene
            .instantiate()
            .expect("Failed to instantiate scene");
        let active_scene_opt = self.active_scene.take();
        if let Some(mut prior_scene) = active_scene_opt {
            self.base_mut().remove_child(&prior_scene);
            prior_scene.queue_free();
        }
        self.base_mut().add_child(&node);
        let gd = Gd::from_instance_id(self.base().instance_id());
        if let Some(mut level) = node.clone().try_cast::<Level>().ok() {
            level
                .signals()
                .complete_level()
                .connect_obj(&gd, Self::on_level_complete);
            level
                .signals()
                .find_bonus()
                .connect_obj(&gd, Self::on_bonus_found);
        } else if let Some(mut message_screen) = node.clone().try_cast::<MessageScreen>().ok() {
            message_screen
                .signals()
                .jump_pressed()
                .connect_obj(&gd, Self::load_next_scene);
        } else if let Some(mut tutorial) = node.clone().try_cast::<Tutorial>().ok() {
            tutorial
                .signals()
                .load_next_scene()
                .connect_obj(&gd, Self::load_next_scene);
        }
        self.active_scene_packed = Some(packed_scene);
        self.active_scene = Some(node);
    }

    #[func]
    fn load_next_scene(&mut self) {
        if self.play_bonus_next {
            self.play_bonus_next = false;
            if let Some(bonus_level) = self.bonus_level.clone() {
                self.load_packed_scene(bonus_level);
                return;
            }
            godot_error!("Missing bonus level!");
        }

        self.scene_index += 1;
        if self.scene_index as usize >= self.scenes.len() {
            self.scene_index = 0;
        }
        self.load_scene();
    }

    #[func]
    fn on_level_complete(&mut self) {
        let scene_name = if (self.scene_index as usize) < self.scenes.len() - 1 {
            "res://messages/finish_level.tscn"
        } else {
            "res://messages/finish_final_level.tscn"
        };
        self.show_message_screen(scene_name);
    }

    #[func]
    fn on_bonus_found(&mut self) {
        self.play_bonus_next = true;
        self.show_message_screen("res://messages/bonus.tscn");
    }

    fn show_message_screen(&self, scene_name: &str) {
        let packed_scene = load::<PackedScene>(scene_name);
        let mut message_screen = packed_scene.instantiate_as::<MessageScreen>();
        let gd = Gd::from_instance_id(self.base().instance_id());
        message_screen
            .signals()
            .jump_pressed()
            .connect_obj(&gd, Self::load_next_scene);

        // Attach to the active scene. This way it will be cleared when we
        // change scenes. Alternatively we could use a separate UI layer.
        self.active_scene
            .clone()
            .expect("Should have an active scene when completing level")
            .add_child(&message_screen);
    }
}
