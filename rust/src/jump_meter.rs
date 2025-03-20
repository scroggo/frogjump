use godot::classes::{Control, IControl, StyleBoxFlat};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Control, tool)]
pub struct JumpMeter {
    ratio: f32,
    style_box: Gd<StyleBoxFlat>,
    #[export]
    size: Vector2,
    #[export]
    bg_color: Color,
    #[export]
    corner_radius: i32,
    #[export]
    border_color: Color,
    #[export]
    border_width: i32,
    #[export]
    shadow_offset: f32,
    #[export]
    shadow_size: i32,
    inner_style_box: Gd<StyleBoxFlat>,
    #[export]
    fill_color: Color,
    //#[export] FIXME: This field is not exported to the editor, though I have
    // tried to fix it with a few guesses:
    // - move it
    // - change its type
    // - replace another export
    // - close and re-open Godot
    offset_y: i32,
    base: Base<Control>,
}

#[godot_api]
impl IControl for JumpMeter {
    fn init(base: Base<Control>) -> Self {
        Self {
            ratio: 0.5,
            style_box: StyleBoxFlat::new_gd(),
            size: Vector2::new(50.0, 25.0),
            bg_color: Color::GREEN,
            corner_radius: 15,
            border_color: Color::WHITE,
            border_width: 7,
            shadow_offset: 2.0,
            shadow_size: 2,
            inner_style_box: StyleBoxFlat::new_gd(),
            fill_color: Color::GREEN,
            offset_y: 30,
            base,
        }
    }

    fn ready(&mut self) {
        let mut style_box = self.style_box.clone();
        style_box.set_bg_color(self.bg_color);
        style_box.set_corner_radius_all(self.corner_radius);
        style_box.set_border_color(self.border_color);
        style_box.set_border_width_all(self.border_width);
        style_box.set_shadow_offset(Vector2::new(self.shadow_offset, self.shadow_offset));
        style_box.set_shadow_size(self.shadow_size);

        let mut inner_style_box = self.inner_style_box.clone();
        inner_style_box.set_corner_radius_all(self.corner_radius);
        inner_style_box.set_bg_color(self.fill_color);
    }

    fn draw(&mut self) {
        // `player.rs` rotates the entire `CharacterBody2D` in order to land on
        // walls and ceilings. Since the `JumpMeter` is a child of that object,
        // it gets rotated, too. In order to draw it consistently below the
        // player, undo the transform and use offsets from the global position.
        let inverse_transform = self.base().get_global_transform().affine_inverse();
        let global_position = self.base().get_global_position()
            + Vector2::new(-self.size.x / 2.0, self.offset_y as f32);

        let style_box = self.style_box.clone();
        let rect = Rect2::new(global_position, self.size);

        let inner_style_box = self.inner_style_box.clone();
        let inset = Vector2::new(self.border_width as f32, self.border_width as f32);
        let mut inner_size = self.size - 2.0 * inset;
        inner_size.x *= self.ratio;
        let inner_rect = Rect2::new(inset + global_position, inner_size);

        self.base_mut().draw_set_transform_matrix(inverse_transform);
        self.base_mut().draw_style_box(&style_box, rect);
        self.base_mut().draw_style_box(&inner_style_box, inner_rect);
    }
}

#[godot_api]
impl JumpMeter {
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
        self.base_mut().queue_redraw();
    }
}
