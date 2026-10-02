use iota_sdk_types::{Argument, Command, Identifier, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct SoulBound {
    soul_bound_package_address: ObjectId,
    manager_cap_type: TypeTag,
}

#[derive(Default)]
pub struct SoulBoundBuilder {
    soul_bound_package_address: Option<ObjectId>,
    manager_cap_type: Option<TypeTag>,
}

impl SoulBoundBuilder {
    pub fn soul_bound_package_address(mut self, value: ObjectId) -> Self {
        self.soul_bound_package_address = Some(value);
        self
    }

    pub fn manager_cap_type(mut self, value: TypeTag) -> Self {
        self.manager_cap_type = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<SoulBound> {
        Ok(SoulBound {
            soul_bound_package_address: self
                .soul_bound_package_address
                .ok_or_else(|| anyhow::anyhow!("soul_bound_package_address is required"))?,
            manager_cap_type: self
                .manager_cap_type
                .ok_or_else(|| anyhow::anyhow!("manager_cap_type is required"))?,
        })
    }
}

impl SoulBound {
    pub fn builder() -> SoulBoundBuilder {
        SoulBoundBuilder::default()
    }

    pub async fn borrow(
        &self,
        builder: &mut TransactionBuilder,
        soul_bound: impl IntoArgument,
    ) -> anyhow::Result<(Argument, Argument)> {
        let soul_bound_arg = builder.obj(soul_bound, true).await?;

        let result = builder.command(Command::new_move_call(
            self.soul_bound_package_address,
            Identifier::new("soul_bound")?,
            Identifier::new("borrow")?,
            vec![self.manager_cap_type.clone()],
            vec![soul_bound_arg],
        ));

        let Argument::Result(cmd_idx) = result else {
            anyhow::bail!("unexpected result from borrow: {result}");
        };

        return Ok((
            Argument::NestedResult(cmd_idx, 0),
            Argument::NestedResult(cmd_idx, 1),
        ));
    }

    pub async fn return_val(
        &self,
        builder: &mut TransactionBuilder,
        soul_bound: impl IntoArgument,
        inner: impl IntoArgument,
        borrow: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let soul_bound_arg = builder.obj(soul_bound, true).await?;
        let inner_arg = builder.obj(inner, true).await?;
        let borrow_arg = builder.obj(borrow, true).await?;

        builder.command(Command::new_move_call(
            self.soul_bound_package_address,
            Identifier::new("soul_bound")?,
            Identifier::new("return_")?,
            vec![self.manager_cap_type.clone()],
            vec![soul_bound_arg, inner_arg, borrow_arg],
        ));

        return Ok(());
    }

    pub async fn share(
        &self,
        builder: &mut TransactionBuilder,
        mcap: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let mcap_arg = builder.obj(mcap, true).await?;

        builder.command(Command::new_move_call(
            self.soul_bound_package_address,
            Identifier::new("soul_bound")?,
            Identifier::new("share")?,
            vec![self.manager_cap_type.clone()],
            vec![mcap_arg],
        ));

        return Ok(());
    }
}
