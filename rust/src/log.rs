// Helper for optional log statements that can be toggled with a boolean.
#[macro_export]
macro_rules! log {
    ($log_tag:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        if $log_tag {
            godot_print!($fmt $(, $args)*)
        }
    };
}
