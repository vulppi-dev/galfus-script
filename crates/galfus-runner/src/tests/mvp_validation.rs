use galfus_core::DiagnosticCodeKind;
use galfus_frontend::{ParserDiagnosticCode, TypeDiagnosticCode};

use crate::check::check_path;
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_project() -> Result<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("galfus-mvp-validation-test-{unique}"));

    fs::create_dir_all(path.as_path())?;

    Ok(path)
}

fn write_file(root: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let path = root.join(name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path.as_path(), text)?;

    Ok(path)
}

#[test]
fn mvp_validation_accepts_core_language_surface() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
type Bytes = [uint8]

const enabled: bool = true
var count: int32 = <int32> 1
var bytes: Bytes = "ok"
var fixed: [int32; 3] = [1, 2, 3]
var values: [int32] = [0, ...fixed, 4]
var last: int32 | null = values[-1]
var point: (int32, bool) = (1, true)

enum<uint8> Mode {
  Off(0),
  On(1),
}

choice MaybeInt {
  Some(int32),
  None,
}

struct Config {
  const id: int32,
  name: Bytes,
  score: int32 = 1,
}

var config = new(Config) { id: 1, name: "main" }
var copied: Config = copy config
var { id, name } = config
var (left, flag) = point
var [head, ...tail] = [1, 2, 3]

fn(stamp) doubled(value: int32): int32 {
  return value + value
}

fn normalize(value: int32 | null): int32 {
  return instanceof value {
    int32 number => number,
    null => 0,
  }
}

fn unwrap(value: MaybeInt): int32 {
  return match value {
    MaybeInt::Some(number) => number,
    MaybeInt::None => 0,
  }
}

fn main(): null {
  var mode: Mode = Mode::On
  var rawMode: uint8 = <uint8> Mode::On
  var maybe: MaybeInt = MaybeInt::Some(doubled(2))
  var answer: int32 = unwrap(maybe)
  var normalized: int32 = normalize(answer)

  for value in values {
    var item: int32 = value
  }

  return
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn mvp_validation_accepts_constraints_decorators_generics_and_ownership() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
type Bytes = [uint8]

fn trim(target: Bytes): Bytes {
  return target
}

fn wrap(target: fn(): null): fn(): null {
  return target
}

fn nullable(target: User | null): User | null {
  return target
}

constraint Named {
  name: Bytes,
  fn label(): Bytes,
}

struct User satisfies Named {
  @trim
  name: Bytes,
  @nullable
  weak manager: User | null,
}

fn User::label(): Bytes {
  return "user"
}

fn identity<T: int | [uint8]>(value: T): T {
  return value
}

fn collect(...values: [int32]): int32 {
  return 0
}

@wrap
fn save(): null {
  return
}

fn main(): null {
  var user = new(User) { name: identity<Bytes>("Ana"), manager: null }
  var label: Bytes = User::label()
  var first: int32 = collect(1, 2, 3)
  var make = (): User => user
  var captured: User = make()
  save()
  return
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors(), "{:?}", result.diagnostics());

    let type_result = result
        .modules()
        .iter()
        .find_map(|module| module.type_result())
        .expect("main module should be typechecked");

    assert!(!type_result.ownership_metadata().weak_fields().is_empty());
    assert!(!type_result.ownership_metadata().captures().is_empty());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn mvp_validation_accepts_local_import_export_project() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import user from "./user"
import { add } from "./math"

fn main(value: user::User): null {
  var total: int32 = add(1, 2)
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
export struct User {
  id: int32,
}
"#,
    )?;

    write_file(
        root.as_path(),
        "math.gfs",
        r#"
export fn add(a: int32, b: int32): int32 {
  return a + b
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(result.modules().len(), 3);

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn mvp_validation_reports_golden_frontend_diagnostics() -> Result<()> {
    let root = temp_project()?;
    let parser_error = write_file(
        root.as_path(),
        "parser_error.gfs",
        r#"
fn call(value: int32): null {
  return
}

fn main(values: [int32]): null {
  call(...values)
  return
}
"#,
    )?;

    let parser_result = check_path(parser_error.as_path())?;

    assert!(parser_result.has_errors());
    assert!(parser_result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == ParserDiagnosticCode::UnexpectedToken.as_code()
            && diagnostic
                .message()
                .contains("expected expression, found `DotDotDot`")
    }));

    let type_error = write_file(
        root.as_path(),
        "type_error.gfs",
        r#"
enum<bool> Mode {
  Off,
  On,
}

struct Node {
  weak parent: Node,
}
"#,
    )?;

    let type_result = check_path(type_error.as_path())?;

    assert!(type_result.has_errors());
    assert!(type_result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidEnumBaseType.as_code()
            && diagnostic
                .message()
                .contains("enum base type must be an integer")
    }));
    assert!(type_result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidWeakFieldType.as_code()
            && diagnostic.message().contains("weak field")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}
