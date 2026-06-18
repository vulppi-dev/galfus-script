use super::*;
use galfus_core::ScopeId;

const BUILTIN_TYPES: &[&str] = &[
    "null", "bool", "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64",
    "float16", "float32", "float64",
];

impl<'a> Resolver<'a> {
    pub(super) fn create_builtin_scope(&mut self) -> ScopeId {
        let scope = self.resolution.add_scope(ScopeKind::Builtin, None, None);

        for name in BUILTIN_TYPES {
            self.declare_builtin_type(scope, name);
        }

        scope
    }

    fn declare_builtin_type(&mut self, scope: ScopeId, name: &str) {
        let Some(declaration) = self.syntax.root() else {
            return;
        };

        if self
            .resolution
            .scope(scope)
            .and_then(|scope| scope.symbol(name))
            .is_some()
        {
            return;
        }

        let symbol = self.resolution.add_symbol(
            SymbolKind::BuiltinType,
            name.to_string(),
            declaration,
            scope,
        );

        if let Some(scope) = self.resolution.scope_mut(scope) {
            scope.insert_symbol(name.to_string(), symbol);
        }
    }
}
