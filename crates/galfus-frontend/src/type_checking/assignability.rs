use galfus_core::TypeId;

use crate::TypeKind;

use super::DeclarationTypeChecker;

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn is_assignable(&self, expected: TypeId, actual: TypeId) -> bool {
        if expected == actual {
            return true;
        }

        let expected_kind = self.layer.table().kind(expected).cloned();
        let actual_kind = self.layer.table().kind(actual).cloned();

        if matches!(expected_kind, Some(TypeKind::Error)) {
            return true;
        }

        if matches!(actual_kind, Some(TypeKind::Error)) {
            return true;
        }

        match (expected_kind, actual_kind) {
            (Some(TypeKind::Union { members }), _) => members.contains(&actual),

            (
                Some(TypeKind::Array {
                    element: expected_element,
                }),
                Some(TypeKind::Array {
                    element: actual_element,
                }),
            ) => self.is_assignable(expected_element, actual_element),

            (
                Some(TypeKind::Array {
                    element: expected_element,
                }),
                Some(TypeKind::FixedArray {
                    element: actual_element,
                    ..
                }),
            ) => self.is_assignable(expected_element, actual_element),

            (
                Some(TypeKind::FixedArray {
                    element: expected_element,
                    size: expected_size,
                }),
                Some(TypeKind::FixedArray {
                    element: actual_element,
                    size: actual_size,
                }),
            ) => {
                expected_size == actual_size && self.is_assignable(expected_element, actual_element)
            }

            _ => false,
        }
    }
}
