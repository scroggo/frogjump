use godot::prelude::*;

mod arrow;
mod dragon_fly;
mod jump_handler;
mod jump_meter;
mod player;
mod tutorial;

struct RustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RustExtension {}
