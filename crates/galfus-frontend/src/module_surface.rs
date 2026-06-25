#[cfg(test)]
mod tests;

use std::collections::HashMap;

use galfus_core::{SymbolId, TypeId};

use crate::{
    ImportedFunctionParameterType, ImportedType, ModuleGraph, SymbolKind, TypeCheckResult, TypeKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSurface {
    exports: Vec<ModuleSurfaceExport>,
    exports_by_name: HashMap<String, usize>,
}

impl ModuleSurface {
    pub fn new(exports: Vec<ModuleSurfaceExport>) -> Self {
        let exports_by_name = exports
            .iter()
            .enumerate()
            .map(|(index, export)| (export.name().to_string(), index))
            .collect();

        Self {
            exports,
            exports_by_name,
        }
    }

    pub fn exports(&self) -> &[ModuleSurfaceExport] {
        self.exports.as_slice()
    }

    pub fn export(&self, name: &str) -> Option<&ModuleSurfaceExport> {
        self.exports_by_name
            .get(name)
            .and_then(|index| self.exports.get(*index))
    }

    pub fn imported_type_for_export(
        &self,
        local_symbol: SymbolId,
        name: &str,
    ) -> Option<ImportedType> {
        let export = self.export(name)?;

        if export.kind().is_type_definition() {
            return Some(ImportedType::NamedLocal {
                symbol: local_symbol,
            });
        }

        export.ty().cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSurfaceExport {
    name: String,
    kind: SymbolKind,
    ty: Option<ImportedType>,
}

impl ModuleSurfaceExport {
    pub fn new(name: String, kind: SymbolKind, ty: Option<ImportedType>) -> Self {
        Self { name, kind, ty }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    pub fn ty(&self) -> Option<&ImportedType> {
        self.ty.as_ref()
    }
}

pub fn build_module_surface(graph: &ModuleGraph, type_result: &TypeCheckResult) -> ModuleSurface {
    let Some(resolution) = graph.resolution() else {
        return ModuleSurface::new(Vec::new());
    };

    let exports = resolution
        .exports()
        .iter()
        .map(|export| {
            let ty = if export.kind().is_type_definition() {
                None
            } else {
                type_result
                    .layer()
                    .symbol_type(export.symbol())
                    .and_then(|ty| transport_type(type_result, ty))
            };

            ModuleSurfaceExport::new(export.name().to_string(), export.kind(), ty)
        })
        .collect();

    ModuleSurface::new(exports)
}

fn transport_type(result: &TypeCheckResult, ty: TypeId) -> Option<ImportedType> {
    match result.layer().table().kind(ty).cloned()? {
        TypeKind::Primitive(primitive) => Some(ImportedType::Primitive(primitive)),

        TypeKind::Array { element } => Some(ImportedType::Array {
            element: Box::new(transport_type(result, element)?),
        }),

        TypeKind::FixedArray { element, size } => Some(ImportedType::FixedArray {
            element: Box::new(transport_type(result, element)?),
            size,
        }),

        TypeKind::Range { element } => Some(ImportedType::Range {
            element: Box::new(transport_type(result, element)?),
        }),

        TypeKind::Tuple { elements } => {
            let elements = elements
                .into_iter()
                .map(|element| transport_type(result, element))
                .collect::<Option<Vec<_>>>()?;

            Some(ImportedType::Tuple { elements })
        }

        TypeKind::Union { members } => {
            let members = members
                .into_iter()
                .map(|member| transport_type(result, member))
                .collect::<Option<Vec<_>>>()?;

            Some(ImportedType::Union { members })
        }

        TypeKind::Function(function) => {
            let parameters = function
                .parameters()
                .iter()
                .map(|parameter| {
                    let ty = transport_type(result, parameter.ty())?;

                    if parameter.is_rest() {
                        return Some(ImportedFunctionParameterType::rest(ty));
                    }

                    if parameter.has_default() {
                        return Some(ImportedFunctionParameterType::with_default(ty));
                    }

                    Some(ImportedFunctionParameterType::new(ty))
                })
                .collect::<Option<Vec<_>>>()?;

            let return_type = Box::new(transport_type(result, function.return_type())?);

            Some(ImportedType::Function {
                parameters,
                return_type,
            })
        }

        _ => None,
    }
}

impl SymbolKind {
    pub fn is_type_definition(self) -> bool {
        matches!(
            self,
            Self::Struct | Self::Enum | Self::Choice | Self::Constraint | Self::TypeAlias
        )
    }
}
