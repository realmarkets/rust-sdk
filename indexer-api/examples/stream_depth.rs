use indexer_api::{Client, ListMarketsParams, Markets, StreamMessage, StreamParams, Topic};

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
        "Streaming market depth for {} ({})\n",
        btc_market.symbol, btc_market.id
    );

    let (handle, mut stream) = client.connect_multiplex_stream(StreamParams::new()).await?;
    handle
        .subscribe(
            "depth",
            [Topic::MarketDepthDiff(Markets::new([&btc_market.id]))],
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
            indexer_api::StreamEventData::MarketDepthDiff(diff) => {
                let best_bid = diff.buys.first().map(|b| b.0).unwrap_or_default();
                let best_ask = diff.sells.first().map(|s| s.0).unwrap_or_default();
                let active_buys = diff.buys.iter().filter(|b| b.1.is_some()).count();
                let active_sells = diff.sells.iter().filter(|s| s.1.is_some()).count();
                println!(
                    "Bid: {} | Ask: {} | Updates: {}B/{}S",
                    best_bid, best_ask, active_buys, active_sells
                );
            }
            indexer_api::StreamEventData::Unknown(value) => {
                println!("Unknown event: {}", json::to_string_pretty(&value)?);
            }
            _ => {}
        }
    }

    Ok(())
}
