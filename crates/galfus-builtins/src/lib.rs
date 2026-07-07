#[cfg(test)]
mod tests;

pub const STD_IO_SOURCE: &str = include_str!("../rich_builtins/io.gfs");
pub const TEXT_SOURCE: &str = include_str!("../rich_builtins/text.gfs");
pub const FORMAT_SOURCE: &str = include_str!("../rich_builtins/format.gfs");
pub const FORMAT_ANSI_SOURCE: &str = include_str!("../rich_builtins/format/ansi.gfs");
pub const BUFFER_SOURCE: &str = include_str!("../rich_builtins/buffer.gfs");

pub static BUILTIN_MODULES: &[(&str, &str)] = &[
    ("std/io", STD_IO_SOURCE),
    ("text", TEXT_SOURCE),
    ("format", FORMAT_SOURCE),
    ("format/ansi", FORMAT_ANSI_SOURCE),
    ("std/buffer", BUFFER_SOURCE),
];

pub fn is_builtin_module(source: &str) -> bool {
    BUILTIN_MODULES.iter().any(|(name, _)| *name == source)
}
