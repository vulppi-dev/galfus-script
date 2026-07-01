use crate::run_source;

#[test]
fn run_source_captures_stdout() {
    let result = run_source(
        r#"
import { println } from 'std/io'

export fn main(args: [[uint8]]): int32 {
  println("hello")
  return 7
}
"#,
        &[],
    );

    assert_eq!(result.error, None);
    assert_eq!(result.exit_code, 7);
    assert_eq!(result.output, "hello\n");
}

#[test]
fn run_source_passes_entry_arguments() {
    let result = run_source(
        r#"
import { println } from 'std/io'

export fn main(args: [[uint8]]): int32 {
  println(<[uint8]>args[0])
  println(<[uint8]>args[1])
  return 3
}
"#,
        &["alpha", "beta"],
    );

    assert_eq!(result.error, None);
    assert_eq!(result.exit_code, 3);
    assert_eq!(result.output, "alpha\nbeta\n");
}

#[test]
fn run_source_exercises_showcase_builtins() {
    let result = run_source(
        r#"
import { println } from 'std/io'
import text from "text"
import ansi from "format/ansi"

export fn main(args: [[uint8]]): int32 {
  println(text::concat("he", "llo"))
  println(ansi::red()::apply("error"))
  return 0
}
"#,
        &[],
    );

    assert_eq!(result.error, None);
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.output, "hello\n\x1b[38;2;220;38;38merror\x1b[0m\n");
}
