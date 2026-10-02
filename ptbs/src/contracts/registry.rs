use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Registry {
    account_registry_package_address: ObjectId,
    registry_object_arg: Input,
    address_registry_object_arg: Input,
}

#[derive(Default)]
pub struct RegistryBuilder {
    account_registry_package_address: Option<ObjectId>,
    registry_object_arg: Option<Input>,
    address_registry_object_arg: Option<Input>,
}

impl RegistryBuilder {
    pub fn account_registry_package_address(mut self, value: ObjectId) -> Self {
        self.account_registry_package_address = Some(value);
        self
    }

    pub fn registry_object_arg(mut self, value: Input) -> Self {
        self.registry_object_arg = Some(value);
        self
    }

    pub fn address_registry_object_arg(mut self, value: Input) -> Self {
        self.address_registry_object_arg = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Registry> {
        Ok(Registry {
            account_registry_package_address: self
                .account_registry_package_address
                .ok_or_else(|| anyhow::anyhow!("account_registry_package_address is required"))?,
            registry_object_arg: self
                .registry_object_arg
                .ok_or_else(|| anyhow::anyhow!("registry_object_arg is required"))?,
            address_registry_object_arg: self
                .address_registry_object_arg
                .ok_or_else(|| anyhow::anyhow!("address_registry_object_arg is required"))?,
        })
    }
}

impl Registry {
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder::default()
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        manager_cap: impl IntoArgument,
        index: u32,
    ) -> anyhow::Result<Argument> {
        let registry_arg = builder.obj(self.registry_object_arg.clone(), true).await?;
        let address_registry_arg = builder
            .obj(self.address_registry_object_arg.clone(), false)
            .await?;
        let cap_arg = builder.obj(manager_cap, true).await?;
        let index_arg = builder.pure(index)?;

        let result = builder.command(Command::new_move_call(
            self.account_registry_package_address,
            Identifier::new("registry")?,
            Identifier::new("new")?,
            vec![coin_type.clone()],
            vec![registry_arg, address_registry_arg, cap_arg, index_arg],
        ));

        return Ok(result);
    }
}
