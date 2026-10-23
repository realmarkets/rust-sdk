use iota_sdk_types::{ObjectId, TypeTag};

/// One dynamic field of a parent object, as listed over gRPC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicFieldInfo {
    pub name: DynamicFieldName,
    pub kind: DynamicFieldKind,
    /// The Move type of the value; for an object field, of the child object.
    pub value_type: String,
    /// The `0x2::dynamic_field::Field` object holding the field.
    pub field_id: ObjectId,
}

/// A dynamic field's name: its Move type and its value as BCS. For an object
/// field this is the name the caller used, not the `Wrapper` it is stored
/// under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicFieldName {
    pub type_tag: TypeTag,
    pub bcs: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicFieldKind {
    /// A value stored inline (`0x2::dynamic_field`).
    Field,
    /// An object stored by ID (`0x2::dynamic_object_field`).
    Object,
}
