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

        match expected_kind {
            Some(TypeKind::Union { members }) => members.contains(&actual),
            _ => false,
        }
    }
}
