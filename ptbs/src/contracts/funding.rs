use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Funding {
    pub funding_package_address: ObjectId,
    pub clock_object_arg: Input,
}

impl Funding {
    pub async fn crank_v1(
        &self,
        builder: &mut TransactionBuilder,
        estimator: impl IntoArgument,
        crank_cap: impl IntoArgument,
        price: impl IntoArgument,
        impact_ask: impl IntoArgument,
        impact_bid: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let estimator_arg = builder.obj(estimator, true).await?;
        let crank_cap_arg = builder.obj(crank_cap, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;
        let price_arg = builder.obj(price, false).await?;
        let impact_ask_arg = builder.pure(impact_ask)?;
        let impact_bid_arg = builder.pure(impact_bid)?;

        let funding = builder.command(Command::new_move_call(
            self.funding_package_address,
            Identifier::new("v1")?,
            Identifier::new("crank")?,
            vec![],
            vec![
                estimator_arg,
                crank_cap_arg,
                clock_arg,
                price_arg,
                impact_ask_arg,
                impact_bid_arg,
            ],
        ));

        return Ok(funding);
    }
}
