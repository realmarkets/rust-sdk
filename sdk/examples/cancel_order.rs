use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use sdk::{
    Client,
    crypto::Signer,
    types::{CancelOrderRequest, Transaction, TransactionPayload},
};

const ACCOUNT: &str = "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a";

// must match the chain id the node reports over `real_chainId`, or the
// transaction is rejected. `real-t-0001` is testnet's, and also what a node
// started from crates/config/src/example.toml reports locally.
const CHAIN_ID: &str = "real-t-0001";

// a locally-run matching engine; testnet is https://matching.api.testnet.real.xyz
const RPC_URL: &str = "http://localhost:1789";

// A private key with the following format could be used as well.
const PRIVKEY: &str = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // load private keys
    let signer: Signer = PRIVKEY.into();

    // instantiate the client
    let client = Client::new(RPC_URL)?;

    // get the coresponding address
    println!("using address: {}", signer.address());

    // our order request
    let cancel_order_request = CancelOrderRequest::builder()
        .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
        .account(ACCOUNT)
        // NOTE: order_id is optional, not setting it will
        // cancel all orders for the given market at once
        .order_id(123456789)
        .build()?;

    // build our transaction payload
    let payload = TransactionPayload::builder()
        // you could also use commands here, if you want to
        // send a batch;
        .command(cancel_order_request)
        .chain_id(CHAIN_ID)
        // microseconds since the Unix epoch: the engine rejects a nonce of 0,
        // one outside its replay window, one too far ahead of its clock, and
        // any value it has already seen for this account
        .nonce(SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64)
        .build()?;

    // build the actual transaction + sign it using
    // our private key
    let tx = Transaction::builder()
        // this is optional
        .client_request_id(uuid::Uuid::new_v4())
        .payload(payload.clone())
        .sign_and_build(&signer)
        .unwrap();

    client.send_transaction(tx).await?;

    return Ok(());
}
