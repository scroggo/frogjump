# Frog Jump

Inspired by the [1-Button Jam](https://itch.io/jam/1-button-jam-2024). Work in progress game as I learn to use the [Godot game engine](https://godotengine.org/). Also one of my first attempts to use Rust in an actual project, thanks to [godot-rust](https://godot-rust.github.io/book/index.html).

## How to play

Go to [scroggo.itch.io/frogjump](https://scroggo.itch.io/frogjump) to play the web version. One button (space bar or mouse click, or touch anywhere on mobile) is all you need! Hold to charge up a jump. Hold the button/fill the bar for a longer jump.

## Exporting to web

General instructions can be found [in the godot-rust book](https://godot-rust.github.io/book/toolchain/export-web.html).

1. `cargo +nightly build -Zbuild-std --target wasm32-unknown-emscripten` # debug build for testing
2. [Test locally](https://godot-rust.github.io/book/toolchain/export-web.html#running-the-webserver).
3. `cargo +nightly build -Zbuild-std --target wasm32-unknown-emscripten --release`
4. Project -> Export -> Web (Runnable) -> Export Project...
5. `cp web_export/anim.png web_export/raw/index.png`[^1]
6. `zip web_export/zipped/<dest>.zip web_export/raw/* -v`

[^1]: `anim.png` is an APNG file. Godot lets you specify an image file to show on the splash screen while loading, but it only supports (non-animated) PNG. However, you can replace the default file with an APNG, and many browsers animate it. (Others should display the first frame as a regular PNG.)
