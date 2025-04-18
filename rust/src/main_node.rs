use godot::classes::InputEvent;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    #[export]
    scenes: Array<Gd<PackedScene>>,
    scene_index: usize,
    active_scene: Option<Gd<Node>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for Main {
    fn init(base: Base<Node>) -> Self {
        Self {
            scenes: Array::<Gd<PackedScene>>::new(),
            scene_index: 0,
            active_scene: None,
            base,
        }
    }

    fn ready(&mut self) {
        self.load_scene();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("jump") {
            self.load_next_scene();
        }
    }
}

#[godot_api]
impl Main {
    fn load_scene(&mut self) {
        if let Some(packed_scene) = self.scenes.get(self.scene_index) {
            godot_print!("Loading scene: {}", self.scene_index);
            let node = packed_scene
                .instantiate()
                .expect("Failed to instantiate scene");
            let active_scene_opt = self.active_scene.take();
            if let Some(mut prior_scene) = active_scene_opt {
                self.base_mut().remove_child(&prior_scene);
                prior_scene.queue_free();
            }
            self.base_mut().add_child(&node);
            self.active_scene = Some(node);
        } else {
            godot_error!("Failed to load scene {}", self.scene_index);
        }
    }

    fn load_next_scene(&mut self) {
        self.scene_index += 1;
        if self.scene_index >= self.scenes.len() {
            self.scene_index = 0;
        }
        self.load_scene();
    }
}
