use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct MockSource {
    pub mock_source_package_address: ObjectId,
    pub clock_object_arg: Input,
}

impl MockSource {
    pub async fn set_value(
        &self,
        builder: &mut TransactionBuilder,
        source: impl IntoArgument,
        price: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let source_arg = builder.obj(source, true).await?;
        let price_arg = builder.obj(price, false).await?;

        builder.command(Command::new_move_call(
            self.mock_source_package_address,
            Identifier::new("mock_source")?,
            Identifier::new("set_value")?,
            vec![],
            vec![source_arg, price_arg],
        ));

        return Ok(());
    }

    pub async fn price(
        &self,
        builder: &mut TransactionBuilder,
        source: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let source_arg = builder.obj(source, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        let price = builder.command(Command::new_move_call(
            self.mock_source_package_address,
            Identifier::new("mock_source")?,
            Identifier::new("price")?,
            vec![],
            vec![source_arg, clock_arg],
        ));

        return Ok(price);
    }
}
