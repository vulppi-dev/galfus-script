#[cfg(test)]
mod tests;

pub const STD_IO_SOURCE: &str = include_str!("../rich_builtins/io.gfs");
pub const BUFFER_SOURCE: &str = include_str!("../rich_builtins/buffer.gfs");
pub const CONSTRAINTS_SOURCE: &str = include_str!("../rich_builtins/constraints.gfs");
pub const RANGE_SOURCE: &str = include_str!("../rich_builtins/range.gfs");

pub const TEXT_SOURCE: &str = include_str!("../rich_builtins/text.gfs");
pub const FORMAT_SOURCE: &str = include_str!("../rich_builtins/format.gfs");
pub const FORMAT_ANSI_SOURCE: &str = include_str!("../rich_builtins/format/ansi.gfs");

pub static BUILTIN_MODULES: &[(&str, &str)] = &[
    ("std/io", STD_IO_SOURCE),
    ("std/buffer", BUFFER_SOURCE),
    ("std/constraints", CONSTRAINTS_SOURCE),
    ("std/range", RANGE_SOURCE),
    ("text", TEXT_SOURCE),
    ("format", FORMAT_SOURCE),
    ("format/ansi", FORMAT_ANSI_SOURCE),
];

pub fn is_builtin_module(source: &str) -> bool {
    BUILTIN_MODULES.iter().any(|(name, _)| *name == source)
}
