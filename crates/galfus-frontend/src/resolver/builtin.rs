use super::*;
use galfus_core::ScopeId;

const BUILTIN_TYPES: &[&str] = &[
    "null", "bool", "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64",
    "float16", "float32", "float64", "int", "uint", "float",
];

struct BuiltinConstraint {
    name: &'static str,
    generic_parameters: &'static [&'static str],
    functions: &'static [&'static str],
}

const BUILTIN_CONSTRAINTS: &[BuiltinConstraint] = &[
    BuiltinConstraint {
        name: "Iterator",
        generic_parameters: &["T", "Item"],
        functions: &["next"],
    },
    BuiltinConstraint {
        name: "Iterable",
        generic_parameters: &["T", "Item", "Iter"],
        functions: &["iter"],
    },
    BuiltinConstraint {
        name: "Comparable",
        generic_parameters: &["Pattern", "Value"],
        functions: &["compare"],
    },
];

impl<'a> Resolver<'a> {
    pub(super) fn create_builtin_scope(&mut self) -> ScopeId {
        let scope = self.resolution.add_scope(ScopeKind::Builtin, None, None);

        for name in BUILTIN_TYPES {
            self.declare_builtin_type(scope, name);
        }

        for constraint in BUILTIN_CONSTRAINTS {
            self.declare_builtin_constraint(scope, constraint);
        }

        scope
    }

    fn declare_builtin_type(&mut self, scope: ScopeId, name: &str) {
        let Some(declaration) = self.syntax.root() else {
            return;
        };

        let name_id = NameId::intern(name);

        if self
            .resolution
            .scope(scope)
            .and_then(|scope| scope.symbol(name_id))
            .is_some()
        {
            return;
        }

        let symbol =
            self.resolution
                .add_symbol(SymbolKind::BuiltinType, name_id, declaration, scope);

        if let Some(scope) = self.resolution.scope_mut(scope) {
            scope.insert_symbol(name_id, symbol);
        }
    }

    fn declare_builtin_constraint(&mut self, scope: ScopeId, constraint: &BuiltinConstraint) {
        let Some(declaration) = self.syntax.root() else {
            return;
        };

        let constraint_name_id = NameId::intern(constraint.name);

        if self
            .resolution
            .scope(scope)
            .and_then(|scope| scope.symbol(constraint_name_id))
            .is_some()
        {
            return;
        }

        let constraint_symbol = self.resolution.add_symbol(
            SymbolKind::Constraint,
            constraint_name_id,
            declaration,
            scope,
        );

        if let Some(scope) = self.resolution.scope_mut(scope) {
            scope.insert_symbol(constraint_name_id, constraint_symbol);
        }

        let member_scope =
            self.resolution
                .add_scope(ScopeKind::Constraint, Some(scope), Some(declaration));

        self.resolution
            .bind_member_scope(constraint_symbol, member_scope);

        for parameter in constraint.generic_parameters {
            let param_name_id = NameId::intern(parameter);
            let symbol = self.resolution.add_symbol(
                SymbolKind::GenericParameter,
                param_name_id,
                declaration,
                member_scope,
            );

            if let Some(scope) = self.resolution.scope_mut(member_scope) {
                scope.insert_symbol(param_name_id, symbol);
            }
        }

        for function in constraint.functions {
            let func_name_id = NameId::intern(function);
            let symbol = self.resolution.add_symbol(
                SymbolKind::ConstraintFunction,
                func_name_id,
                declaration,
                member_scope,
            );

            if let Some(scope) = self.resolution.scope_mut(member_scope) {
                scope.insert_symbol(func_name_id, symbol);
            }
        }
    }
}
