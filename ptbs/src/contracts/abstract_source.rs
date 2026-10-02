use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct AbstractSource {
    pub clock_object_arg: Input,
}

impl AbstractSource {
    /// Read a `Price` from any authorized source. `price_module` is the module exposing the
    /// entrypoint — a dedicated `read` for the older source packages, the module defining the
    /// source struct for Lazer ones — resolved by the caller from the source's on-chain type.
    pub async fn read_price(
        &self,
        builder: &mut TransactionBuilder,
        source_package_address: ObjectId,
        price_module: &str,
        source: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let source_arg = builder.obj(source, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        let value = builder.command(Command::new_move_call(
            source_package_address,
            Identifier::new(price_module)?,
            Identifier::new("price")?,
            vec![],
            vec![source_arg, clock_arg],
        ));

        return Ok(value);
    }
}
