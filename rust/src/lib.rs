use godot::prelude::*;

mod alligator;
mod arrow;
mod button_hint;
mod direction;
mod fly;
mod jump_handler;
mod jump_meter;
mod landing_surface;
mod level;
mod log;
mod main_node;
mod math;
mod message_screen;
mod player;
mod steal_enter;
mod test_alligator;
mod toucan;
mod tutorial;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
