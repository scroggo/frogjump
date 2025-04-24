use godot::prelude::*;
use std::ops::Not;

#[derive(PartialEq, GodotConvert, Var, Export, Clone, Copy)]
#[godot(via=GString)]
pub enum Direction {
    Left,
    Right,
}

impl Default for Direction {
    fn default() -> Self {
        // Platformers traditionally play to the right.
        Self::Right
    }
}

impl Not for Direction {
    fn not(self) -> Self::Output {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    type Output = Self;
}
