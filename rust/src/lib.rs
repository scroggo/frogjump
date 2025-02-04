use godot::prelude::*;

mod alligator;
mod arrow;
mod dragon_fly;
mod jump_handler;
mod jump_meter;
mod level;
mod math;
mod player;
mod test_alligator;
mod toucan;
mod tutorial;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
