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
