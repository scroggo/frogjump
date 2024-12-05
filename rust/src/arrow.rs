use godot::classes::{Control, IControl};
use godot::global::deg_to_rad;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Control, tool)]
struct Arrow {
    #[export]
    end: Vector2,
    #[export]
    color: Color,
    #[export]
    width: f32,
    #[export]
    chevron_degrees: i32,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Arrow {
    fn init(base: Base<Control>) -> Self {
        Self {
            end: Vector2 { x: 100.0, y: 100.0 },
            color: Color::REBECCA_PURPLE,
            width: -1.0,
            chevron_degrees: 90,
            base,
        }
    }

    fn draw(&mut self) {
        let end = self.end;
        let color = self.color;
        let width = self.width;
        self.base_mut()
            .draw_line_ex(Vector2::ZERO, end, color)
            .antialiased(true)
            .width(width)
            .done();
        // Draw the chevron. Choose the length based on the total length of the arrow.
        // TODO: Or let the user specify?
        let length = self.end.length() / 6.0;
        let chevron_base = (Vector2::ZERO - self.end).normalized() * length;
        let angle = deg_to_rad(self.chevron_degrees as f64) as f32;
        let chevron_a = chevron_base.rotated(-angle) + self.end;
        let chevron_b = chevron_base.rotated(angle) + self.end;
        self.base_mut()
            .draw_line_ex(end, chevron_a, color)
            .antialiased(true)
            .width(width)
            .done();
        self.base_mut()
            .draw_line_ex(end, chevron_b, color)
            .antialiased(true)
            .width(width)
            .done();
    }
}
