use galfus_core::DiagnosticCodeKind;

use crate::check::check_path;
use crate::diagnostic::CheckDiagnosticCode;
use anyhow::Result;
use galfus_frontend::TypeDiagnosticCode;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_project() -> Result<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("galfus-runner-test-{unique}"));

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

mod named_imports;
mod namespace_imports;
mod relative_imports;
