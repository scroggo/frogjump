use crate::level::Level;
use crate::message_screen::MessageScreen;
use godot::classes::InputEvent;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    /// List of levels and other scenes (e.g. title screen) to play, in order.
    #[export]
    scenes: Array<Gd<PackedScene>>,
    scene_index: usize,
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
        if event.is_action_pressed("jump") {
            if self.play_bonus_next {
                self.play_bonus_next = false;
                if let Some(bonus_level) = self.bonus_level.clone() {
                    self.load_packed_scene(bonus_level);
                    return;
                }
                godot_error!("Missing bonus level!");
            }
            self.load_next_scene();
        } else if event.is_action_pressed("RELOAD") {
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
        if let Some(packed_scene) = self.scenes.get(self.scene_index) {
            godot_print!("Loading scene: {}", self.scene_index);
            self.load_packed_scene(packed_scene);
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
        if let Some(mut level) = node.clone().try_cast::<Level>().ok() {
            let gd = Gd::from_instance_id(self.base().instance_id());
            level
                .signals()
                .complete_level()
                .connect_obj(&gd, Self::on_level_complete);
            level
                .signals()
                .find_bonus()
                .connect_obj(&gd, Self::on_bonus_found);
        }
        self.active_scene_packed = Some(packed_scene);
        self.active_scene = Some(node);
    }

    fn load_next_scene(&mut self) {
        self.scene_index += 1;
        if self.scene_index >= self.scenes.len() {
            self.scene_index = 0;
        }
        self.load_scene();
    }

    #[func]
    fn on_level_complete(&mut self) {
        let scene_name = if self.scene_index < self.scenes.len() - 1 {
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
        // Note: This cast is not strictly necessary, but it matches the intent.
        let message_screen = packed_scene.instantiate_as::<MessageScreen>();

        // Attach to the active scene. This way it will be cleared when we
        // change scenes. Alternatively we could use a separate UI layer.
        self.active_scene
            .clone()
            .expect("Should have an active scene when completing level")
            .add_child(&message_screen);
    }
}
