use godot::prelude::*;

mod jump_handler;
mod jump_meter;
mod player;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
