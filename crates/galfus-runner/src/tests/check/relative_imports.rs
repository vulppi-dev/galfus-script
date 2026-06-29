use super::*;

#[test]
fn check_path_loads_relative_imported_modules() -> Result<()> {
    let root = temp_project()?;
    let main = write_file(
        root.as_path(),
        "main.gfs",
        r#"
        import user from "./user"

        fn main(): null {
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
fn check_path_accepts_named_import_from_exported_symbol() -> Result<()> {
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
