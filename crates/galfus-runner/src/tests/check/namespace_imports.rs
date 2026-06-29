use super::*;

#[test]
fn check_path_reports_missing_relative_import_module() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import missing from "./missing"

        fn main(): null {
            return
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::ImportModuleNotFound.as_code()
            && diagnostic.message().contains("not found")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_rejects_non_gfs_relative_import_target() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import data from "./data.json"

fn main(): null {
  return
}
"#,
    )?;

    write_file(root.as_path(), "data.json", "{}")?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::UnsupportedImportTarget.as_code()
            && diagnostic
                .message()
                .contains("must resolve to a .gfs source file")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_path_accepts_namespace_import_from_exported_function() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import user from "./user"

fn main(): null {
  user::create()
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
export fn create(): null {
  return
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.modules().len(), 2);

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_function_call() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import math from "./math"

        var value: int32 = math::add(true, 2)

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "math.gfs",
        r#"
        export fn add(a: int32, b: int32): int32 {
            return a
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `int32`, got `bool`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_namespace_import_from_private_function() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import user from "./user"

fn main(): null {
  user::create()
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
fn create(): null {
  return
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::MissingExport.as_code()
            && diagnostic.message().contains("does not export `create`")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_path_accepts_namespace_import_from_exported_type() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import user from "./user"

fn main(value: user::User): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
export struct User {
  id: int64,
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());
    assert_eq!(result.modules().len(), 2);

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_type_path() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import user from "./user"

        fn identity(value: user::User): user::User {
            return value
        }

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        export struct User {
            id: int64,
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_struct_field_access() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import user from "./user"

        fn read(value: user::User): int64 {
            return value.id
        }

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        export struct User {
            id: int64,
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_enum_variant() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import status from "./status"

        var current: status::Status = status::Status::Ready

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "status.gfs",
        r#"
        export enum Status {
            Ready,
            Done,
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_choice_constructor() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import result from "./result"

        var value: result::Result = result::Result::Ok(1)

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "result.gfs",
        r#"
        export choice Result {
            Ok(int32),
            Err([int8]),
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_alias() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import ids from "./ids"

        var id: ids::UserId = 1

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "ids.gfs",
        r#"
        export type UserId = int32
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_function_stamp() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import math from "./math"

        var value: int32 = math::make(1)

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "math.gfs",
        r#"
        export fn(stamp) make(value: int32): int32 {
            return value
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_namespace_imported_anchor_function() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import user from "./user"

        fn rename(value: user::User): user::User {
            return user::User::rename(value, "ada")
        }

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
        export struct User {
            name: [int8],
        }

        export fn User::rename(self: User, name: [int8]): User {
            return self
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_accepts_namespace_imported_constraint_application() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import contracts from "./contracts"

        struct User satisfies contracts::Named {
            name: [uint8],
        }

        fn User::label(): [uint8] {
            return "Ana"
        }

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "contracts.gfs",
        r#"
        export constraint Named {
            name: [uint8],
            fn label(): [uint8],
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_namespace_imported_constraint_generic_argument_count() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import contracts from "./contracts"

        struct User satisfies contracts::Boxed {
            value: int64,
        }

        fn main(): null {
            return
        }
        "#,
    )?;

    write_file(
        root.as_path(),
        "contracts.gfs",
        r#"
        export constraint Boxed<T> {
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str()
            == TypeDiagnosticCode::ConstraintGenericArgumentCountMismatch.as_code()
            && diagnostic
                .message()
                .contains("constraint `Boxed` expects 1 generic argument")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_namespace_import_from_private_type() -> Result<()> {
    let root = temp_project()?;

    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
import user from "./user"

fn main(value: user::User): null {
  return
}
"#,
    )?;

    write_file(
        root.as_path(),
        "user.gfs",
        r#"
struct User {
  id: int64,
}
"#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == CheckDiagnosticCode::MissingExport.as_code()
            && diagnostic.message().contains("does not export `User`")
    }));

    fs::remove_dir_all(root)?;
    Ok(())
}
