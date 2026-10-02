use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Fee {
    exchange_package_address: ObjectId,
    exchange_object_arg: Input,
}

#[derive(Default)]
pub struct FeeBuilder {
    exchange_package_address: Option<ObjectId>,
    exchange_object_arg: Option<Input>,
}

impl FeeBuilder {
    pub fn exchange_package_address(mut self, value: ObjectId) -> Self {
        self.exchange_package_address = Some(value);
        self
    }

    pub fn exchange_object_arg(mut self, value: Input) -> Self {
        self.exchange_object_arg = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Fee> {
        Ok(Fee {
            exchange_package_address: self
                .exchange_package_address
                .ok_or_else(|| anyhow::anyhow!("exchange_package_address is required"))?,
            exchange_object_arg: self
                .exchange_object_arg
                .ok_or_else(|| anyhow::anyhow!("exchange_object_arg is required"))?,
        })
    }
}

impl Fee {
    pub fn builder() -> FeeBuilder {
        FeeBuilder::default()
    }

    pub async fn realize_change(
        &self,
        builder: &mut TransactionBuilder,
        fee_config: impl IntoArgument,
        nonce: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let fee_config_arg = builder.obj(fee_config, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let nonce_arg = builder.pure(nonce)?;

        Ok(builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("fee")?,
            Identifier::new("realize_change")?,
            vec![],
            vec![fee_config_arg, exchange_arg, nonce_arg],
        )))
    }
}
