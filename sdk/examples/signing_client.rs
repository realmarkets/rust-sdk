use std::error::Error;

use sdk::{
    SigningClient,
    crypto::Signer,
    types::{CancelOrderRequest, Command, SubmitOrderRequest},
};

const ACCOUNT: &str = "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a";

// A private key with the following format could be used as well.
const PRIVKEY: &str = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // load private keys
    let signer: Signer = PRIVKEY.into();

    // instantiate the client
    let client = SigningClient::new_testnet(signer.clone())?;

    // get the coresponding address
    println!("using address: {}", signer.address());

    // our cancel order request
    let cancel_order_request = Command::CancelOrder(
        CancelOrderRequest::builder()
            .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
            .account(ACCOUNT)
            // NOTE: order_id is optional, not setting it will
            // cancel all orders for the given market at once
            .order_id(123456789)
            .build()?,
    );

    // our order request
    let submit_order_request = Command::SubmitOrder(
        SubmitOrderRequest::builder()
            .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
            .account(ACCOUNT)
            .time_in_force(sdk::types::TimeInForce::Fok)
            .buy()
            .limit_order("112065.78")
            .quantity("0.01")
            .build()?,
    );

    println!(
        "{:?}",
        client
            .submit_batch([cancel_order_request, submit_order_request], None)
            .await
    );

    return Ok(());
}
