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

    assert_eq!(result, "123");
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

  var res: int32 = 0
  match parsed {
    format::ParseResult::Ok(value) => {
      if value {
        res = 1
      } else {
        res = 0
      }
    },
    format::ParseResult::Err(error) => {
      res = 0
    },
  }
  return res
}
"#,
    )?;

    assert_eq!(result, "1");
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

  var res: int32 = 7
  match parsed {
    format::ParseResult::Ok(value) => {
      res = 0
    },
    format::ParseResult::Err(error) => {
      res = 7
    },
  }
  return res
}
"#,
    )?;

    assert_eq!(result, "7");
    Ok(())
}

#[test]
fn format_parse_float64_standard_decimal_success() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_float64_standard_decimal",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<float64>("-0.1")

  var res: int32 = 0
  match parsed {
    format::ParseResult::Ok(value) => {
      res = 1
    },
    format::ParseResult::Err(error) => {
      res = 0
    },
  }
  return res
}
"#,
    )?;

    assert_eq!(result, "1");
    Ok(())
}

#[test]
fn format_parse_float64_rejects_leading_dot() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_float64_leading_dot",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<float64>(".1")

  var res: int32 = 1
  match parsed {
    format::ParseResult::Ok(value) => {
      res = 0
    },
    format::ParseResult::Err(error) => {
      res = 1
    },
  }
  return res
}
"#,
    )?;

    assert_eq!(result, "1");
    Ok(())
}

#[test]
fn format_parse_float64_rejects_exponent() -> anyhow::Result<()> {
    let result = run_main(
        "galfus_format_parse_float64_exponent",
        r#"
import format from "format"

export fn main(args: [[uint8]]): int32 {
  var parsed = format::parse<float64>("1e2")

  var res: int32 = 1
  match parsed {
    format::ParseResult::Ok(value) => {
      res = 0
    },
    format::ParseResult::Err(error) => {
      res = 1
    },
  }
  return res
}
"#,
    )?;

    assert_eq!(result, "1");
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
