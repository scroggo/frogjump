use godot::prelude::*;

mod alligator;
mod arrow;
mod fly;
mod jump_handler;
mod jump_meter;
mod landing_surface;
mod level;
mod math;
mod message_screen;
mod player;
mod test_alligator;
mod toucan;
mod tutorial;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
