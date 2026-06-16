use crate::{GraphPhase, ScopeKind, SymbolKind, SyntaxNodeKind, parse, resolve};
use galfus_core::{SourceFile, SourceId};

mod block;
mod export;
mod function;
mod import;
mod reference;
mod scope;
mod symbol;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "main.gfs".to_string(), text.to_string())
}
