use std::time::Duration;

use indexer_api::{
    Client, ControlMessage, ListMarketsParams, Scope, StreamMessage, StreamParams, Topic,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new_testnet()?;

    let markets = client.list_markets(ListMarketsParams::new()).await?;
    let btc_market = markets
        .data
        .iter()
        .find(|m| m.symbol == "BTC-USDT")
        .expect("BTC-USDT market not found");

    println!(
        "Multiplex stream, subscribing to orders and trades for {} ({})\n",
        btc_market.symbol, btc_market.id
    );

    // open a heartbeat-only connection with no permanent filters, then subscribe dynamically
    let (handle, mut stream) = client
        .connect_multiplex_stream(StreamParams::new().heartbeat(true))
        .await?;

    // the handle is cloneable and Send, so subscriptions can be managed from a separate
    // task while the main task keeps reading
    let market_id = btc_market.id.clone();
    tokio::spawn(async move {
        // one subscription can span several topics; they share this tag and are removed together
        let topics = [
            Topic::Orders(Scope::new().markets([&market_id])),
            Topic::Trades(Scope::new().markets([&market_id])),
        ];
        if let Err(e) = handle.subscribe("btc-orders-trades", topics).await {
            eprintln!("subscribe failed: {e}");
        }
    });

    // size the liveness watchdog off the server's heartbeat cadence: if no frame (not even a
    // heartbeat) arrives within ~2 intervals, treat the connection as dead
    let health = client.health_check().await?;
    let idle_timeout = Duration::from_secs(health.stream.heartbeat_interval_secs.max(1) as u64 * 2);

    loop {
        match stream.next_message_timeout(idle_timeout).await {
            Ok(Some(StreamMessage::Event(event))) => {
                // events carry no tag — route by payload (event.data's market_id/account_id)
                println!("event {} (sv={:?})", event.name, event.state_version);
            }
            Ok(Some(StreamMessage::Control(ControlMessage::SubscribeAck { tag }))) => {
                println!("subscribed: {tag}");
            }
            Ok(Some(StreamMessage::Control(ControlMessage::SubscribeError { tag, message }))) => {
                eprintln!("subscribe error [{tag}]: {message}");
            }
            Ok(Some(StreamMessage::Control(other))) => {
                println!("control: {other:?}");
            }
            Ok(None) => break, // connection closed cleanly
            Err(indexer_api::Error::StreamIdle { timeout }) => {
                // in a real bot: reconnect via client.reconnect_multiplex_stream(params, &handle)
                eprintln!("no frame for {timeout:?} — connection idle, would reconnect");
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
