use super::*;

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
fn check_accepts_for_over_fixed_array_literal() {
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
        struct Numbers satisfies Iterable<Numbers, int32, NumbersIterator> {
          values: [int32],
        }

        struct NumbersIterator satisfies Iterator<NumbersIterator, int32> {
          values: [int32],
          index: int32,
        }

        fn Numbers::iter(self: Numbers): NumbersIterator {
          return NumbersIterator {
            values: self.values,
            index: 0,
          }
        }

        fn NumbersIterator::next(self: NumbersIterator): int32 | null {
          return self.values[self.index]
        }

        fn main(): null {
          var nums = Numbers {
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
        struct Numbers satisfies Iterable<Numbers, int32, NumbersIterator> {
          values: [int32],
        }

        struct NumbersIterator satisfies Iterator<NumbersIterator, int32> {
          values: [int32],
          index: int32,
        }

        fn Numbers::iter(self: Numbers): NumbersIterator {
          return NumbersIterator {
            values: self.values,
            index: 0,
          }
        }

        fn NumbersIterator::next(self: NumbersIterator): int32 | null {
          return self.values[self.index]
        }

        fn main(): null {
          var nums = Numbers {
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
struct Iter {}

struct Source satisfies Iterable<Source, int32, Iter> {}

fn Source::iter(self: Source): Iter {
  return Iter {}
}

fn main(): null {
  var source = Source {}

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
        struct BadIterator satisfies Iterator<BadIterator, int32> {}

        fn BadIterator::next(self: BadIterator): bool {
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
fn check_accepts_builtin_comparable_constraint() {
    let (_source, _graph, result) = check_source(
        r#"
        struct Pattern satisfies Comparable<Pattern, [uint8]> {}

        fn Pattern::compare(self: Pattern, value: [uint8]): bool {
          return true
        }
        "#,
    );

    assert!(!result.has_errors());
}
