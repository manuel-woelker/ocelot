use ocelot_ast::type_kind::TypeKind;
use ocelot_base::shared_string::SharedString;

/// Converts one native signature type into a user-facing type label.
pub fn native_type_label(type_kind: TypeKind) -> SharedString {
    match type_kind {
        TypeKind::Any => "any".into(),
        TypeKind::Boolean => "bool".into(),
        TypeKind::String => "string".into(),
        TypeKind::Unresolved => "unresolved".into(),
    }
}
