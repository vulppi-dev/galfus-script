use super::*;

#[test]
fn check_path_typechecks_named_imported_function_call() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { add } from "./math"

        var value: int32 = add(true, 2)

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
fn check_path_accepts_named_imported_function_call() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { add } from "./math"

        var value: int32 = add(1, 2)

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

    assert!(!result.has_errors());
    assert!(
        result
            .modules()
            .iter()
            .all(|module| module.type_result().is_some())
    );

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_named_imported_struct_field_access() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { User } from "./user"

        fn read(value: User): int64 {
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
fn check_path_typechecks_named_imported_enum_variant() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Status } from "./status"

        var current: Status = Status::Ready

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
fn check_path_typechecks_named_imported_choice_constructor() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Result } from "./result"

        var value: Result = Result::Ok(1)

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
fn check_path_accepts_named_imported_choice_match() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Result } from "./result"

        fn unwrap(value: Result): int32 {
            return match value {
                Result::Ok(inner) => inner,
                Result::Err(message) => 0,
            }
        }

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
            Err([uint8]),
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_named_imported_choice_match_payload_mismatch() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Result } from "./result"

        fn unwrap(value: Result): int32 {
            return match value {
                Result::Ok(true) => 1,
                Result::Err(message) => 0,
            }
        }

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
            Err([uint8]),
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidMatchPatternType.as_code()
            && diagnostic.message().contains("got `bool`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_named_imported_choice_match_non_exhaustive() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Result } from "./result"

        fn unwrap(value: Result): int32 {
            return match value {
                Result::Ok(inner) => inner,
            }
        }

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
            Err([uint8]),
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::NonExhaustiveMatch.as_code()
            && diagnostic.message().contains("missing `Err`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_accepts_named_imported_constraint_application() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Named } from "./contracts"

        struct User satisfies Named {
            name: [uint8],
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
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(!result.has_errors());

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_named_imported_constraint_missing_field() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { Named } from "./contracts"

        struct User satisfies Named {
            id: int64,
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
        }
        "#,
    )?;

    let result = check_path(main.as_path())?;

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::MissingConstraintField.as_code()
            && diagnostic.message().contains("missing field `name`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_typechecks_named_imported_alias() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { UserId } from "./ids"

        var id: UserId = true

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
fn check_path_reports_named_imported_struct_unknown_field() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { User } from "./user"

        fn read(value: User): int64 {
            return value.missing
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

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::UnknownMember.as_code()
            && diagnostic.message().contains("has no member `missing`")
    }));

    fs::remove_dir_all(root)?;

    Ok(())
}

#[test]
fn check_path_reports_named_import_from_private_symbol() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import { User } from "./user"

        fn main(value: User): null {
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
