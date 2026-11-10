use std::future::Future;

use anyhow::{Context, anyhow, bail, ensure};
use iota_sdk_grpc_client::{GrpcClient, read_mask_fields::DynamicFieldField};
use iota_sdk_grpc_types::v1::dynamic_field::{
    DynamicField, dynamic_field::DynamicFieldKind as ProtoKind,
};
use iota_sdk_types::{Address, ObjectId, TypeTag};
use serde::de::DeserializeOwned;

use super::{
    Timed,
    object_args::{ObjectExt, ObjectReads},
};
use crate::dynamic_field::{DynamicFieldInfo, DynamicFieldKind, DynamicFieldName};

/// Fields per gRPC list page; the server caps it at 1000.
const LIST_PAGE_SIZE: u32 = 1000;

/// Dynamic-field reads.
pub(super) trait FieldReads {
    /// Every dynamic field of `parent`, across all pages.
    fn fetch_dynamic_fields(
        &self,
        parent: ObjectId,
    ) -> impl Future<Output = anyhow::Result<Vec<DynamicFieldInfo>>> + Send;

    /// The value of `parent`'s field named by the BCS `name` of `name_type`,
    /// found by deriving the field's object ID, or `None` when there is no
    /// such field.
    fn fetch_dynamic_field<V: DeserializeOwned>(
        &self,
        parent: ObjectId,
        name_type: &TypeTag,
        name: &[u8],
    ) -> impl Future<Output = anyhow::Result<Option<V>>> + Send;
}

impl FieldReads for GrpcClient {
    async fn fetch_dynamic_fields(
        &self,
        parent: ObjectId,
    ) -> anyhow::Result<Vec<DynamicFieldInfo>> {
        let fields = self
            .dynamic_fields(parent)
            .read_mask([
                DynamicFieldField::KIND,
                DynamicFieldField::NAME,
                DynamicFieldField::VALUE_TYPE,
                DynamicFieldField::FIELD_ID,
                DynamicFieldField::FIELD_OBJECT,
            ])
            .page_size(LIST_PAGE_SIZE)
            .collect(None)
            .timed()
            .await?
            .into_inner();
        return fields.iter().map(DynamicFieldInfo::try_from).collect();
    }

    async fn fetch_dynamic_field<V: DeserializeOwned>(
        &self,
        parent: ObjectId,
        name_type: &TypeTag,
        name: &[u8],
    ) -> anyhow::Result<Option<V>> {
        let field_id = parent.derive_dynamic_child_id(name_type, name);
        let Some(object) = self.fetch_object(field_id).await? else {
            return Ok(None);
        };
        let field = object.move_struct()?;
        let field_type = field.struct_tag();
        ensure!(
            field_type.is_dynamic_field() && field_type.type_params().first() == Some(name_type),
            "object {field_id} is not a dynamic field named by {name_type}: it is a {field_type}"
        );

        // `Field { id, name, value }`: the value follows the 32-byte id and the name
        let contents = field.contents();
        let value_start = ObjectId::LENGTH + name.len();
        ensure!(
            contents.get(ObjectId::LENGTH..value_start) == Some(name),
            "dynamic field {field_id} holds a different name"
        );
        let value = bcs::from_bytes::<V>(&contents[value_start..]).with_context(|| {
            format!(
                "dynamic field {field_id} does not decode as {}",
                std::any::type_name::<V>()
            )
        })?;
        return Ok(Some(value));
    }
}

impl TryFrom<&DynamicField> for DynamicFieldInfo {
    type Error = anyhow::Error;

    fn try_from(field: &DynamicField) -> anyhow::Result<DynamicFieldInfo> {
        let kind = match field.kind() {
            ProtoKind::Field => DynamicFieldKind::Field,
            ProtoKind::Object => DynamicFieldKind::Object,
            other => bail!("dynamic field of unknown kind {other:?}"),
        };
        let field_object = field
            .field_object
            .as_ref()
            .context("dynamic field listed without its field object")?
            .object()?;
        let field_type = field_object
            .struct_tag()
            .context("dynamic field object is not a Move object")?;
        let stored_name = field_type
            .type_params()
            .first()
            .ok_or_else(|| anyhow!("dynamic field type {field_type} has no name type"))?;
        let type_tag = match kind {
            DynamicFieldKind::Field => stored_name.clone(),
            DynamicFieldKind::Object => stored_name.object_field_name()?,
        };
        let bcs = field
            .name
            .as_ref()
            .context("dynamic field listed without its name")?
            .as_bytes()
            .to_vec();
        let field_id = field
            .field_id
            .as_ref()
            .context("dynamic field listed without its ID")?
            .object_id()?;

        return Ok(DynamicFieldInfo {
            name: DynamicFieldName { type_tag, bcs },
            kind,
            value_type: field.value_type().to_string(),
            field_id,
        });
    }
}

trait ObjectFieldName {
    /// The name type of an object field, stored under
    /// `0x2::dynamic_object_field::Wrapper<Name>`. The wrapper's single field
    /// gives it the same BCS as `Name`.
    fn object_field_name(&self) -> anyhow::Result<TypeTag>;
}

impl ObjectFieldName for TypeTag {
    fn object_field_name(&self) -> anyhow::Result<TypeTag> {
        if let TypeTag::Struct(tag) = self
            && tag.address() == Address::FRAMEWORK
            && tag.module().as_str() == "dynamic_object_field"
            && tag.name().as_str() == "Wrapper"
            && let [name] = tag.type_params()
        {
            return Ok(name.clone());
        }
        bail!("dynamic object field name {self} is not a Wrapper")
    }
}
