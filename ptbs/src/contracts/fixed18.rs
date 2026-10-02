use iota_sdk_types::{Argument, Command, Identifier, ObjectId};
use move_core_types::u256::U256;

use crate::builder::TransactionBuilder;

#[derive(Clone)]
pub struct Fixed18 {
    pub fixed18_package_address: ObjectId,
}

impl Fixed18 {
    #[allow(clippy::wrong_self_convention)]
    pub async fn from_raw_u256(
        &self,
        builder: &mut TransactionBuilder,
        raw_value: U256,
    ) -> anyhow::Result<Argument> {
        let raw_value_arg = builder.pure(raw_value)?;

        let value = builder.command(Command::new_move_call(
            self.fixed18_package_address,
            Identifier::new("fixed18")?,
            Identifier::new("from_raw_u256")?,
            vec![],
            vec![raw_value_arg],
        ));

        return Ok(value);
    }
}
