use iota_sdk_types::{Command, Identifier, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Insurance {
    pub exchange_package_address: ObjectId,
}

impl Insurance {
    /// Permissionless cleanup: drains at most `max_entries` expired entries from the fund's
    /// own cooldown queue. Takes no market, so it reaches a fund no market is bound to.
    pub async fn prune_fund_cooldown_queue(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        config: impl IntoArgument,
        clock: impl IntoArgument,
        max_entries: u64,
    ) -> anyhow::Result<()> {
        let config_arg = builder.obj(config, true).await?;
        let clock_arg = builder.obj(clock, false).await?;
        let max_entries_arg = builder.pure(max_entries)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("insurance")?,
            Identifier::new("prune_fund_cooldown_queue")?,
            vec![coin_type.clone()],
            vec![config_arg, clock_arg, max_entries_arg],
        ));

        return Ok(());
    }
}
