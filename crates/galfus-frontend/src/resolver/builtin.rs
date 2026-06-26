use super::*;
use galfus_core::ScopeId;

const BUILTIN_TYPES: &[&str] = &[
    "null", "bool", "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64",
    "float16", "float32", "float64",
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

    fn declare_builtin_constraint(&mut self, scope: ScopeId, constraint: &BuiltinConstraint) {
        let Some(declaration) = self.syntax.root() else {
            return;
        };

        if self
            .resolution
            .scope(scope)
            .and_then(|scope| scope.symbol(constraint.name))
            .is_some()
        {
            return;
        }

        let constraint_symbol = self.resolution.add_symbol(
            SymbolKind::Constraint,
            constraint.name.to_string(),
            declaration,
            scope,
        );

        if let Some(scope) = self.resolution.scope_mut(scope) {
            scope.insert_symbol(constraint.name.to_string(), constraint_symbol);
        }

        let member_scope =
            self.resolution
                .add_scope(ScopeKind::Constraint, Some(scope), Some(declaration));

        self.resolution
            .bind_member_scope(constraint_symbol, member_scope);

        for parameter in constraint.generic_parameters {
            let symbol = self.resolution.add_symbol(
                SymbolKind::GenericParameter,
                parameter.to_string(),
                declaration,
                member_scope,
            );

            if let Some(scope) = self.resolution.scope_mut(member_scope) {
                scope.insert_symbol(parameter.to_string(), symbol);
            }
        }

        for function in constraint.functions {
            let symbol = self.resolution.add_symbol(
                SymbolKind::ConstraintFunction,
                function.to_string(),
                declaration,
                member_scope,
            );

            if let Some(scope) = self.resolution.scope_mut(member_scope) {
                scope.insert_symbol(function.to_string(), symbol);
            }
        }
    }
}
