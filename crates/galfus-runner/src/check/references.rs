use super::*;

impl ModuleLoader {
    pub(super) fn collect_named_imported_path_types(
        &self,
        module_index: usize,
        surfaces: &[ModuleSurface],
        imported_types: &mut ImportedSurfaceTypes,
    ) {
        let imports = self.module_imports(module_index);
        let named_imports = imports
            .iter()
            .filter(|import| {
                import.kind == ImportKind::Named && is_resolvable_import(import.source.as_str())
            })
            .collect::<Vec<_>>();

        if named_imports.is_empty() {
            return;
        }

        for path in self.path_records(module_index) {
            let Some(root_segment) = path.segments.first() else {
                continue;
            };

            let Some(import) = named_imports
                .iter()
                .find(|import| import.local_name == root_segment.name)
            else {
                continue;
            };

            let Some(imported_name) = import.imported_name.as_deref() else {
                continue;
            };

            let target_path = if is_builtin_import(import.source.as_str()) {
                PathBuf::from(import.source.as_str())
            } else {
                resolve_relative_import(self.modules[module_index].path(), import.source.as_str())
            };

            let Ok(target_path) = normalize_existing_path(target_path.as_path()) else {
                continue;
            };

            let Some(target_index) = self.module_by_path.get(target_path.as_path()).copied() else {
                continue;
            };

            let member_name = path.segments[1..]
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("::");

            let Some(imported_type) = surfaces[target_index]
                .imported_member_path_type_for_named_export(
                    import.local_symbol,
                    imported_name,
                    &member_name,
                )
            else {
                continue;
            };

            imported_types.insert_path_type(path.node, imported_type);
        }
    }

    pub(super) fn collect_namespace_imported_path_types(
        &self,
        module_index: usize,
        surfaces: &[ModuleSurface],
        imported_types: &mut ImportedSurfaceTypes,
    ) {
        let imports = self.module_imports(module_index);
        let namespace_imports = imports
            .iter()
            .filter(|import| {
                import.kind == ImportKind::Namespace && is_resolvable_import(import.source.as_str())
            })
            .collect::<Vec<_>>();

        if namespace_imports.is_empty() {
            return;
        }

        for path in self.path_records(module_index) {
            let Some(root_segment) = path.segments.first() else {
                continue;
            };

            let Some(import) = namespace_imports
                .iter()
                .find(|import| import.local_name == root_segment.name)
            else {
                continue;
            };

            let target_path = if is_builtin_import(import.source.as_str()) {
                PathBuf::from(import.source.as_str())
            } else {
                resolve_relative_import(self.modules[module_index].path(), import.source.as_str())
            };

            let Ok(target_path) = normalize_existing_path(target_path.as_path()) else {
                continue;
            };

            let Some(target_index) = self.module_by_path.get(target_path.as_path()).copied() else {
                continue;
            };

            imported_types.extend(imported_surface_types_for_namespace(
                &surfaces[target_index],
                import.local_symbol,
            ));

            let exported_name = path.segments[1..]
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("::");

            let Some(imported_type) = surfaces[target_index]
                .imported_path_type_for_export(import.local_symbol, &exported_name)
            else {
                if let Some(imported_constraint) =
                    surfaces[target_index].imported_constraint_for_export(exported_name.as_str())
                {
                    imported_types.insert_path_constraint(path.node, imported_constraint);
                }

                continue;
            };

            imported_types.insert_path_type(path.node, imported_type);

            if let Some(imported_constraint) =
                surfaces[target_index].imported_constraint_for_export(exported_name.as_str())
            {
                imported_types.insert_path_constraint(path.node, imported_constraint);
            }

            if let Some(imported_choice) =
                surfaces[target_index].imported_choice_for_export(exported_name.as_str())
            {
                imported_types.insert_path_choice(path.node, imported_choice);
            }
        }
    }

    pub(super) fn named_type_records(&self, module_index: usize) -> Vec<NamedTypeCheckRecord> {
        let mut records = Vec::new();

        let Some(root) = self.modules[module_index].graph().syntax().root() else {
            return records;
        };

        self.collect_named_type_records(module_index, root, &mut records);

        records
    }

    fn collect_named_type_records(
        &self,
        module_index: usize,
        node: NodeId,
        records: &mut Vec<NamedTypeCheckRecord>,
    ) {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(syntax_node) = syntax.node(node) else {
            return;
        };

        if syntax_node.kind() == SyntaxNodeKind::NamedType {
            if let Some(identifier) = syntax.first_child_of_kind(node, SyntaxNodeKind::Identifier) {
                records.push(NamedTypeCheckRecord {
                    node,
                    name: self.node_text(module_index, identifier),
                });
            }

            return;
        }

        for child in syntax_node.children() {
            self.collect_named_type_records(module_index, *child, records);
        }
    }

    fn path_records(&self, module_index: usize) -> Vec<PathCheckRecord> {
        let mut records = Vec::new();

        let Some(root) = self.modules[module_index].graph().syntax().root() else {
            return records;
        };

        self.collect_path_records(module_index, root, &mut records);

        records
    }

    fn collect_path_records(
        &self,
        module_index: usize,
        node: NodeId,
        records: &mut Vec<PathCheckRecord>,
    ) {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(syntax_node) = syntax.node(node) else {
            return;
        };

        match syntax_node.kind() {
            SyntaxNodeKind::PathExpression => {
                let segments = self.path_expression_segments(module_index, node);

                if segments.len() >= 2 {
                    records.push(PathCheckRecord { node, segments });
                }

                return;
            }

            SyntaxNodeKind::Path => {
                let segments = self.type_path_segments(module_index, node);

                if segments.len() >= 2 {
                    records.push(PathCheckRecord { node, segments });
                }

                return;
            }

            _ => {}
        }

        for child in syntax_node.children() {
            self.collect_path_records(module_index, *child, records);
        }
    }

    fn path_expression_segments(
        &self,
        module_index: usize,
        expression: NodeId,
    ) -> Vec<PathSegmentRecord> {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(expression_node) = syntax.node(expression) else {
            return Vec::new();
        };

        match expression_node.kind() {
            SyntaxNodeKind::NameExpression => {
                let Some(identifier) =
                    syntax.first_child_of_kind(expression, SyntaxNodeKind::Identifier)
                else {
                    return Vec::new();
                };

                vec![PathSegmentRecord {
                    name: self.node_text(module_index, identifier),
                    node: identifier,
                }]
            }

            SyntaxNodeKind::PathExpression => {
                let Some(target) = syntax.child(expression, 0) else {
                    return Vec::new();
                };

                let Some(member) = syntax.child(expression, 1) else {
                    return Vec::new();
                };

                let mut segments = self.path_expression_segments(module_index, target);

                segments.push(PathSegmentRecord {
                    name: self.node_text(module_index, member),
                    node: member,
                });

                segments
            }

            _ => Vec::new(),
        }
    }

    fn type_path_segments(&self, module_index: usize, path: NodeId) -> Vec<PathSegmentRecord> {
        let syntax = self.modules[module_index].graph().syntax();

        let Some(path_node) = syntax.node(path) else {
            return Vec::new();
        };

        path_node
            .children()
            .iter()
            .filter_map(|child| {
                let child_node = syntax.node(*child)?;

                if child_node.kind() != SyntaxNodeKind::Identifier {
                    return None;
                }

                Some(PathSegmentRecord {
                    name: self.node_text(module_index, *child),
                    node: *child,
                })
            })
            .collect()
    }

    fn node_text(&self, module_index: usize, node: NodeId) -> String {
        let Some(syntax_node) = self.modules[module_index].graph().syntax().node(node) else {
            return String::new();
        };

        self.modules[module_index]
            .source()
            .slice(syntax_node.span())
            .unwrap_or("")
            .to_string()
    }

    pub(super) fn validate_namespace_import_references(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.module_imports(module_index);

            let namespace_imports = imports
                .iter()
                .filter(|import| {
                    import.kind == ImportKind::Namespace
                        && is_resolvable_import(import.source.as_str())
                })
                .collect::<Vec<_>>();

            if namespace_imports.is_empty() {
                continue;
            }

            let paths = self.path_records(module_index);

            for path in paths {
                let Some(root_segment) = path.segments.first() else {
                    continue;
                };

                let Some(import) = namespace_imports
                    .iter()
                    .find(|import| import.local_name == root_segment.name)
                else {
                    continue;
                };

                let target_path = if is_builtin_import(import.source.as_str()) {
                    PathBuf::from(import.source.as_str())
                } else {
                    resolve_relative_import(
                        self.modules[module_index].path(),
                        import.source.as_str(),
                    )
                };

                let Ok(target_path) = normalize_existing_path(target_path.as_path()) else {
                    continue;
                };

                let Some(target_index) = self.module_by_path.get(target_path.as_path()).copied()
                else {
                    continue;
                };

                let Some(target_resolution) = self.modules[target_index].graph().resolution()
                else {
                    continue;
                };

                let exported_name = path.segments[1..]
                    .iter()
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");

                if self.target_exports_path(target_resolution, exported_name.as_str()) {
                    continue;
                }

                let span = self.modules[module_index]
                    .graph()
                    .syntax()
                    .node(path.segments[1].node)
                    .map(|node| node.span())
                    .unwrap_or_else(|| self.modules[module_index].source().span());

                self.diagnostics.push(Diagnostic::error_with_message(
                    CheckDiagnosticCode::MissingExport,
                    format!(
                        "module `{}` does not export `{}`",
                        import.source, exported_name
                    ),
                    span,
                ));
            }
        }
    }

    fn target_exports_path(&self, resolution: &ResolutionLayer, exported_name: &str) -> bool {
        if resolution.export_by_name(exported_name).is_some() {
            return true;
        }

        let Some((owner_name, member_name)) = exported_name.rsplit_once("::") else {
            return false;
        };

        let Some(owner) = resolution
            .export_by_name(owner_name)
            .and_then(|export| resolution.export_record(export))
        else {
            return false;
        };

        let Some(member_scope) = resolution.member_scope(owner.symbol()) else {
            return false;
        };

        resolution
            .scope(member_scope)
            .and_then(|scope| scope.symbol(member_name))
            .is_some()
    }
}

pub(crate) fn check_path(path: impl AsRef<Path>) -> Result<CheckResult> {
    let mut loader = ModuleLoader::default();

    loader.check_entry(path.as_ref())?;

    Ok(CheckResult {
        modules: loader.modules,
        diagnostics: loader.diagnostics,
    })
}
