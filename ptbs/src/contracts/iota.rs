use std::str::FromStr;

use iota_sdk_types::{Argument, Command, Identifier, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Object {
    market_type_tag: TypeTag,
}

#[derive(Default)]
pub struct ObjectBuilder {
    market_type_tag: Option<TypeTag>,
}

impl ObjectBuilder {
    pub fn market_type_tag(mut self, value: TypeTag) -> Self {
        self.market_type_tag = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Object> {
        Ok(Object {
            market_type_tag: self
                .market_type_tag
                .ok_or_else(|| anyhow::anyhow!("market_type_tag is required"))?,
        })
    }
}

impl Object {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::default()
    }

    pub async fn id(
        &self,
        builder: &mut TransactionBuilder,
        market: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let market_arg = builder.obj(market, false).await?;

        let id = builder.command(Command::new_move_call(
            ObjectId::from_str("0x2")?,
            Identifier::new("object")?,
            Identifier::new("id")?,
            vec![self.market_type_tag.clone()],
            vec![market_arg],
        ));

        return Ok(id);
    }
}
