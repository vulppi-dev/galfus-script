use crate::{check_workspace, compile_workspace_to_image};
use galfus_runtime::Runtime;
use galfus_target::WebTarget;
use std::fs;
use std::path::PathBuf;

#[test]
fn format_parse_int32_success() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_int32",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<int32>("123")

  return match parsed {
    format::ParseResult::Ok(value) => value,
    format::ParseResult::Err(error) => 0,
  }
}
"#,
    )?;

    assert_eq!(format!("{result:?}"), "123");
    Ok(())
}

#[test]
fn format_parse_bool_success() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_bool",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<bool>("true")

  return match parsed {
    format::ParseResult::Ok(value) => {
      if value {
        return 1
      }

      return 0
    },
    format::ParseResult::Err(error) => 0,
  }
}
"#,
    )?;

    assert_eq!(format!("{result:?}"), "1");
    Ok(())
}

#[test]
fn format_parse_invalid_returns_err() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_invalid",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<int32>("abc")

  return match parsed {
    format::ParseResult::Ok(value) => value,
    format::ParseResult::Err(error) => 7,
  }
}
"#,
    )?;

    assert_eq!(format!("{result:?}"), "7");
    Ok(())
}

fn run_main(name: &str, main_source: &str) -> anyhow::Result<String> {
    let root = temp_workspace(name);
    let src = root.join("src");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src)?;

    fs::write(
        root.join("galfus.toml"),
        r#"
[module]
name = "format-parse-test"
target = "app"
entry = "src/main.gfs"
"#,
    )?;

    fs::write(src.join("main.gfs"), main_source)?;

    let check_result = check_workspace(&root)?;
    println!(
        "TEST_DIAGNOSTICS: {:?}",
        check_result.check_result().diagnostics()
    );
    assert!(
        !check_result.has_errors(),
        "workspace check failed: {:?}",
        check_result.check_result().diagnostics()
    );

    let image = compile_workspace_to_image(&check_result)?;
    let module_name = image.name.clone();

    let target = WebTarget::new();
    let mut runtime = Runtime::new(Box::new(target));
    runtime.loader().load(image);

    let args: Vec<Vec<u8>> = Vec::new();
    let result = runtime
        .run_entry(module_name.as_str(), "main", &args)
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    let _ = fs::remove_dir_all(&root);

    Ok(format!("{result:?}"))
}

fn temp_workspace(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
}
