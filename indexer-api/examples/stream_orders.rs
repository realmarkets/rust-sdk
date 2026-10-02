use indexer_api::{Client, ListMarketsParams, Scope, StreamMessage, StreamParams, Topic};

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
        "Streaming orders for {} ({})\n",
        btc_market.symbol, btc_market.id
    );

    let (handle, mut stream) = client.connect_multiplex_stream(StreamParams::new()).await?;
    handle
        .subscribe(
            "orders",
            [Topic::Orders(Scope::new().markets([&btc_market.id]))],
        )
        .await?;

    while let Some(message) = stream.next_message().await? {
        let event = match message {
            StreamMessage::Event(event) => event,
            StreamMessage::Control(control) => {
                println!("control: {:?}", control);
                continue;
            }
        };

        if let Some(ref version) = event.state_version {
            print!("[{}] ", version);
        }

        match event.data {
            indexer_api::StreamEventData::Order(order) => {
                println!("{}@{}", order.quantity, order.price.unwrap_or_default());
            }
            indexer_api::StreamEventData::Unknown(value) => {
                println!("Unknown event: {}", json::to_string_pretty(&value)?);
            }
            _ => {}
        }
    }

    Ok(())
}
