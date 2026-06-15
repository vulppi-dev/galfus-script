use super::*;
use galfus_core::SourceId;

mod create;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "main.gfs".to_string(), text.to_string())
}
