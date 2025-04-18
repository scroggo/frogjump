use godot::prelude::*;

mod alligator;
mod arrow;
mod button_hint;
mod fly;
mod jump_handler;
mod jump_meter;
mod landing_surface;
mod level;
mod main_node;
mod math;
mod message_screen;
mod player;
mod test_alligator;
mod toucan;
mod tutorial;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
