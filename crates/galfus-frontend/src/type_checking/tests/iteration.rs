use super::*;
use crate::ResolverDiagnosticCode;

#[test]
fn check_accepts_for_over_dynamic_array() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: int32 = value
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_array_literal() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for value in [1, 2, 3] {
    var copied: int32 = value
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_string_literal_as_uint8_array() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for byte in "Ana" {
    var copied: uint8 = byte
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_empty_string_literal() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(): null {
  for byte in "" {
    var copied: uint8 = byte
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_for_over_non_array() {
    let source = source(
        r#"
fn main(): null {
  for value in 10 {
    var copied: int32 = value
  }

  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidIterableType.as_code()
            && diagnostic
                .message()
                .contains("for iterable must satisfy `Iterable`, got `int32`")
    }));
}

#[test]
fn check_binds_for_binding_type_from_dynamic_array() {
    let (source, graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: int32 = value
  }

  return
}
"#,
    );

    let binding =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::ForBinding, "value").unwrap();

    let ty = result.layer().node_type(binding).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_reports_for_binding_type_mismatch_in_body() {
    let source = source(
        r#"
fn main(values: [int32]): null {
  for value in values {
    var copied: bool = value
  }

  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::TypeMismatch.as_code()
            && diagnostic
                .message()
                .contains("expected `bool`, got `int32`")
    }));
}

#[test]
fn check_accepts_for_over_iterable_struct() {
    let (_source, _graph, result) = check_source(
        r#"
        constraint Iterator<T, Item> {
          fn next(self): Item | null
        }

        constraint Iterable<T, Item, Iter> {
          fn iter(self): Iter
        }

        struct Numbers satisfies Iterable<Numbers, int32, NumbersIterator> {
          values: [int32],
        }

        struct NumbersIterator satisfies Iterator<NumbersIterator, int32> {
          values: [int32],
          index: int32,
        }

        fn Numbers::iter(self): NumbersIterator {
          return new(NumbersIterator) {
            values: self.values,
            index: 0,
          }
        }

        fn NumbersIterator::next(self): int32 | null {
          return self.values[self.index]
        }

        fn main(): null {
          var nums = new(Numbers) {
            values: [1, 2, 3],
          }

          for value in nums {
            var copied: int32 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_for_binding_type_from_iterable_struct() {
    let (source, graph, result) = check_source(
        r#"
        constraint Iterator<T, Item> {
          fn next(self): Item | null
        }

        constraint Iterable<T, Item, Iter> {
          fn iter(self): Iter
        }

        struct Numbers satisfies Iterable<Numbers, int32, NumbersIterator> {
          values: [int32],
        }

        struct NumbersIterator satisfies Iterator<NumbersIterator, int32> {
          values: [int32],
          index: int32,
        }

        fn Numbers::iter(self): NumbersIterator {
          return new(NumbersIterator) {
            values: self.values,
            index: 0,
          }
        }

        fn NumbersIterator::next(self): int32 | null {
          return self.values[self.index]
        }

        fn main(): null {
          var nums = new(Numbers) {
            values: [1, 2, 3],
          }

          for value in nums {
            var copied: int32 = value
          }

          return
        }
        "#,
    );

    let binding =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::ForBinding, "value").unwrap();

    let ty = result.layer().node_type(binding).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_reports_iterable_with_non_iterator_iter_type() {
    let source = source(
        r#"
constraint Iterator<T, Item> {
  fn next(self): Item | null
}

constraint Iterable<T, Item, Iter> {
  fn iter(self): Iter
}

struct Iter {}

struct Source satisfies Iterable<Source, int32, Iter> {}

fn Source::iter(self): Iter {
  return new(Iter) {}
}

fn main(): null {
  var source = new(Source) {}

  for item in source {
    var copied: int32 = item
  }

  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidIterableType.as_code()
            && diagnostic
                .message()
                .contains("for iterable must satisfy `Iterable`")
    }));
}

#[test]
fn check_reports_iterator_next_return_type_mismatch() {
    let source = source(
        r#"
        constraint Iterator<T, Item> {
          fn next(self): Item | null
        }

        struct BadIterator satisfies Iterator<BadIterator, int32> {}

        fn BadIterator::next(self): bool {
          return true
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `next` expected")
    }));
}

#[test]
fn check_accepts_comparable_constraint() {
    let (_source, _graph, result) = check_source(
        r#"
        constraint Comparable<Pattern, Value> {
          fn compare(self, value: Value): bool
        }

        struct Pattern satisfies Comparable<Pattern, [uint8]> {}

        fn Pattern::compare(self, value: [uint8]): bool {
          return true
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_direct_builtin_constraint_without_import() {
    let source = source(
        r#"
struct Pattern satisfies Comparable<Pattern, [uint8]> {}

fn Pattern::compare(self, value: [uint8]): bool {
  return true
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::RestrictedBuiltinSymbol.as_code()
            && diagnostic.message().contains("Comparable")
    }));
}

#[test]
fn check_reports_iterable_iter_return_type_mismatch() {
    let source = source(
        r#"
constraint Iterator<T, Item> {
  fn next(self): Item | null
}

constraint Iterable<T, Item, Iter> {
  fn iter(self): Iter
}

struct Iter {}

struct Source satisfies Iterable<Source, int32, Iter> {}

fn Source::iter(self): bool {
  return true
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `iter` expected")
    }));
}

#[test]
fn check_reports_iterator_next_item_type_mismatch() {
    let source = source(
        r#"
constraint Iterator<T, Item> {
  fn next(self): Item | null
}

struct BadIterator satisfies Iterator<BadIterator, int32> {}

fn BadIterator::next(self): bool | null {
  return true
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `next` expected")
    }));
}

#[test]
fn check_reports_builtin_comparable_return_type_mismatch() {
    let source = source(
        r#"
constraint Comparable<Pattern, Value> {
  fn compare(self, value: Value): bool
}

struct Pattern satisfies Comparable<Pattern, [uint8]> {}

fn Pattern::compare(self, value: [uint8]): int32 {
  return 0
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::ConstraintFunctionTypeMismatch.as_code()
            && diagnostic.message().contains("function `compare` expected")
    }));
}

#[test]
fn check_reports_for_over_iterator_without_iterable() {
    let source = source(
        r#"
constraint Iterator<T, Item> {
  fn next(self): Item | null
}

struct Counter satisfies Iterator<Counter, int32> {}

fn Counter::next(self): int32 | null {
  return 1
}

fn main(): null {
  var counter = new(Counter) {}

  for value in counter {
    var copied: int32 = value
  }

  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidIterableType.as_code()
            && diagnostic
                .message()
                .contains("for iterable must satisfy `Iterable`")
    }));
}

#[test]
fn check_accepts_for_over_exclusive_range() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 1..10 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_quantity_range_with_step() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 1::10%2 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_for_binding_type_from_range() {
    let (source, graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 1..10 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let binding =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::ForBinding, "value").unwrap();

    let ty = result.layer().node_type(binding).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int64))
    );
}

#[test]
fn check_reports_range_assigned_to_int() {
    let source = source(
        r#"
        fn main(): null {
          var seq: int32 = 1::10
          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.message().contains("expected `int32`")
            && diagnostic.message().contains("range<int64>")
    }));
}

#[test]
fn check_accepts_for_over_quantity_range() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 1::10 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_for_over_float_quantity_range() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 1.0::3%0.5 {
            var copied: float64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_descending_exclusive_range() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 10..1 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_accepts_negative_range_step() {
    let (_source, _graph, result) = check_source(
        r#"
        fn main(): null {
          for value in 10::4%-2 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_reports_float_exclusive_range() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1.0..10.0 {
            var copied: float64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic
                .message()
                .contains("range operand must be integer literal")
    }));
}

#[test]
fn check_reports_empty_exclusive_range() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1..1 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic.message().contains("range must not be empty")
    }));
}

#[test]
fn check_reports_non_positive_range_count() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1::0 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic
                .message()
                .contains("range count must be greater than zero")
    }));
}

#[test]
fn check_reports_negative_range_count() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1::-1 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic
                .message()
                .contains("range count must be greater than zero")
    }));
}

#[test]
fn check_reports_range_overflow() {
    let source = source(
        r#"
        fn main(): null {
          for value in 9223372036854775800::20 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic.message().contains("range end overflows int64")
    }));
}

#[test]
fn check_reports_zero_range_step() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1::2%0 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic.message().contains("range step must not be zero")
    }));
}

#[test]
fn check_reports_mismatched_range_step_family() {
    let source = source(
        r#"
        fn main(): null {
          for value in 1::2%0.5 {
            var copied: int64 = value
          }

          return
        }
        "#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());
    assert!(!resolve_result.has_errors());

    let graph = resolve_result.into_graph();
    let result = check_declaration_types(&source, &graph);

    assert!(result.has_errors());
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == TypeDiagnosticCode::InvalidRangeOperandType.as_code()
            && diagnostic
                .message()
                .contains("range step must have the same numeric family as start")
    }));
}

#[test]
fn check_accepts_ignored_for_binding() {
    let (_source, _graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for _ in values {
    var copied: int32 = 1
  }

  return
}
"#,
    );

    assert!(!result.has_errors());
}

#[test]
fn check_binds_for_index_as_int32() {
    let (source, graph, result) = check_source(
        r#"
fn main(values: [int32]): null {
  for value, index in values {
    var copied: int32 = value
    var position: int32 = index
  }

  return
}
"#,
    );

    let index =
        find_node_by_kind_and_text(&source, &graph, SyntaxNodeKind::Identifier, "index").unwrap();

    let ty = result.layer().node_type(index).unwrap();

    assert_eq!(
        result.layer().table().kind(ty),
        Some(&TypeKind::Primitive(PrimitiveType::Int32))
    );
}

#[test]
fn check_ignored_for_binding_does_not_create_referenceable_symbol() {
    let source = source(
        r#"
fn main(values: [int32]): null {
  for _ in values {
    var copied: int32 = _
  }

  return
}
"#,
    );

    let parse_result = parse(&source);
    assert!(!parse_result.has_errors());

    let resolve_result = resolve(&source, parse_result.into_graph());

    assert!(resolve_result.has_errors());
    assert!(resolve_result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code().as_str() == ResolverDiagnosticCode::UnresolvedName.as_code()
            && diagnostic.message().contains("unresolved name `_`")
    }));
}
