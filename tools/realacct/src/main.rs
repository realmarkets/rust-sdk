//! `realacct` — interactive tool to inspect and manage a Real Markets trading account.
//!
//! Loads a trading account from an IOTA private key, shows its on-chain status
//! (address, account, collateral balance, VIP membership) and offers the
//! relevant next action: create the account if none exists, or deposit and
//! withdraw funds if one does.
//!
//! # Usage
//!
//! ```bash
//! cargo run -p realacct -- --private-key iotaprivkey1... [--network testnet] [--account-index 0]
//! ```

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crypto::Signer;
use dex::{
    account::{AccountBalance, AccountDetails, Accounts, AccountsConfig, CollateralInfo},
    contracts_artifacts::Artifacts,
    iota::{IotaNetwork, build_iota_client},
};
use inquire::{Confirm, CustomType, Select, Text};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::fmt;
use std::future::Future;
use std::io::Write;
use std::time::Duration;

/// Indexer to fetch contract artifacts from, paired with the IOTA chain its
/// contracts are deployed on.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum Network {
    Testnet,
}

impl Network {
    /// Indexer base URL used to fetch the latest contract artifacts.
    fn indexer_url(self) -> &'static str {
        return match self {
            Network::Testnet => "https://indexer.api.testnet.real.xyz",
        };
    }

    /// IOTA chain the contracts are deployed on.
    fn iota_network(self) -> IotaNetwork {
        return match self {
            Network::Testnet => IotaNetwork::Testnet,
        };
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "realacct",
    about = "Inspect and manage a Real Markets trading account",
    version,
    term_width = 100
)]
struct Cli {
    /// Your IOTA private key (iotaprivkey1...).
    #[arg(short, long)]
    private_key: String,

    /// Indexer to fetch contract artifacts from (its contracts are on IOTA testnet).
    #[arg(short, long, value_enum, default_value_t = Network::Testnet)]
    network: Network,

    /// Account index in the registry.
    #[arg(short = 'i', long, default_value_t = 0)]
    account_index: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let artifacts = with_spinner(
        &format!("Loading artifacts from {}", cli.network.indexer_url()),
        Artifacts::from_indexer(cli.network.indexer_url()),
    )
    .await
    .context("failed to load artifacts from indexer")?;
    let config: AccountsConfig = artifacts.into();

    let client = with_spinner(
        "Connecting to the IOTA network",
        build_iota_client(&cli.network.iota_network()),
    )
    .await
    .context("failed to connect to the IOTA network")?;

    let signer: Signer = cli.private_key.clone().into();
    let address = signer.address();

    let accounts = with_spinner("Initializing account interface", async {
        Accounts::new(client, config, signer).await
    })
    .await
    .context("failed to initialize accounts interface")?;

    let collateral = with_spinner("Reading collateral metadata", accounts.collateral_info())
        .await
        .context("failed to read collateral metadata")?;

    return run(&accounts, &address, cli.account_index, &collateral).await;
}

/// Status-then-action loop: refresh on-chain state, print it, offer the
/// relevant actions, and repeat until the user quits.
async fn run(
    accounts: &Accounts,
    address: &str,
    account_index: u32,
    collateral: &CollateralInfo,
) -> Result<()> {
    loop {
        let (existing, is_vip, balance) = with_spinner("Reading account status", async {
            let existing = accounts
                .get_existing_opt(account_index)
                .await
                .context("failed to look up account")?;
            let is_vip = accounts
                .is_vip_sender(address)
                .await
                .context("failed to check VIP status")?;
            // Balance lives on the account object, so it only exists once the
            // account has been created.
            let balance = match &existing {
                Some(details) => Some(
                    accounts
                        .account_balance(&details.account_object_id)
                        .await
                        .context("failed to read account balance")?,
                ),
                None => None,
            };
            anyhow::Ok((existing, is_vip, balance))
        })
        .await?;

        print_status(
            address,
            account_index,
            existing.as_ref(),
            is_vip,
            balance.as_ref(),
            collateral,
        );

        let actions = match existing {
            None => vec![Action::Create, Action::Refresh, Action::Quit],
            Some(_) => vec![
                Action::Deposit,
                Action::Withdraw,
                Action::Refresh,
                Action::Quit,
            ],
        };
        let action = Select::new("Action:", actions).prompt()?;

        match action {
            Action::Create => create_account(accounts, account_index).await?,
            // `existing` is always `Some` when Deposit / Withdraw are offered.
            Action::Deposit => {
                if let Some(details) = &existing {
                    deposit(accounts, details, collateral, is_vip).await?;
                }
            }
            Action::Withdraw => {
                if let Some(details) = &existing {
                    withdraw(accounts, details, collateral, address).await?;
                }
            }
            Action::Refresh => {}
            Action::Quit => return Ok(()),
        }
    }
}

fn print_status(
    address: &str,
    account_index: u32,
    existing: Option<&AccountDetails>,
    is_vip: bool,
    balance: Option<&AccountBalance>,
    collateral: &CollateralInfo,
) {
    println!();
    println!("  address  {address}");
    println!("  index    {account_index}");
    println!(
        "  asset    {} ({} decimals)",
        collateral.symbol, collateral.decimals
    );
    match existing {
        Some(details) => {
            println!("  account  {}", details.account_object_id);
            println!("  id       {}", details.account_id);
        }
        None => println!("  account  no account created yet"),
    }
    if let Some(balance) = balance {
        println!(
            "  balance  {} {}",
            fmt_amount(balance.value, balance.is_neg, collateral.decimals),
            collateral.symbol,
        );
    }
    println!("  VIP      {}", if is_vip { "yes" } else { "no" });
    println!();
}

async fn create_account(accounts: &Accounts, account_index: u32) -> Result<()> {
    let confirm = Confirm::new(&format!("Create a new account at index {account_index}?"))
        .with_default(true)
        .prompt()?;
    if !confirm {
        return Ok(());
    }

    println!("Creating account...");
    let (details, tx_digest) = accounts
        .create(account_index)
        .await
        .context("failed to create account")?;

    println!("\n✓ Account created");
    println!("  account object id: {}", details.account_object_id);
    println!("  account id:        {}", details.account_id);
    println!("  manager cap id:    {}", details.manager_cap_id);
    println!("  transaction:       {tx_digest}");
    return Ok(());
}

async fn deposit(
    accounts: &Accounts,
    details: &AccountDetails,
    collateral: &CollateralInfo,
    is_vip: bool,
) -> Result<()> {
    let mut methods = vec![DepositMethod::Wallet];
    if is_vip {
        methods.push(DepositMethod::VipFaucet);
    }
    let method = Select::new("Deposit method:", methods).prompt()?;

    // Prompt in human-readable units of the collateral asset and scale to the
    // chain's raw representation ourselves using the coin's decimals.
    let amount = CustomType::<Decimal>::new(&format!("Amount to deposit ({}):", collateral.symbol))
        .with_error_message("Please enter a valid amount.")
        .prompt()?;
    if amount <= Decimal::ZERO {
        println!("Amount must be greater than zero.");
        return Ok(());
    }

    let tx_digest = match method {
        DepositMethod::Wallet => {
            // `deposit` takes raw smallest units: scale by 10^decimals.
            let raw = match to_base_units(amount, collateral.decimals) {
                Ok(raw) => raw,
                Err(msg) => {
                    println!("{} {msg}", collateral.symbol);
                    return Ok(());
                }
            };
            println!(
                "Depositing {amount} {} ({raw} base units) from wallet...",
                collateral.symbol
            );
            accounts
                .deposit(&details.account_object_id, raw)
                .await
                .context("deposit failed")?
        }
        DepositMethod::VipFaucet => {
            // The faucet's `vip_deposit` scales whole units by the coin's
            // decimals on-chain, so it only mints whole-token amounts.
            if !amount.fract().is_zero() {
                println!(
                    "VIP faucet mint supports whole {} amounts only.",
                    collateral.symbol
                );
                return Ok(());
            }
            let Some(whole) = amount.to_u64() else {
                println!("Amount is too large.");
                return Ok(());
            };
            println!("Minting {whole} {} via VIP faucet...", collateral.symbol);
            accounts
                .vip_deposit(&details.account_object_id, whole)
                .await
                .context("VIP deposit failed")?
        }
    };

    println!("\n✓ Deposit successful");
    println!("  amount:      {amount} {}", collateral.symbol);
    println!("  transaction: {tx_digest}");
    return Ok(());
}

async fn withdraw(
    accounts: &Accounts,
    details: &AccountDetails,
    collateral: &CollateralInfo,
    address: &str,
) -> Result<()> {
    let amount =
        CustomType::<Decimal>::new(&format!("Amount to withdraw ({}):", collateral.symbol))
            .with_error_message("Please enter a valid amount.")
            .prompt()?;
    if amount <= Decimal::ZERO {
        println!("Amount must be greater than zero.");
        return Ok(());
    }
    let raw = match to_base_units(amount, collateral.decimals) {
        Ok(raw) => raw,
        Err(msg) => {
            println!("{} {msg}", collateral.symbol);
            return Ok(());
        }
    };

    let recipient = Text::new("Recipient address:")
        .with_default(address)
        .prompt()?;

    // Strict by default: fail if free collateral is short, rather than silently
    // withdrawing less than requested.
    let best_effort = Confirm::new(
        "Best effort? (withdraw what's available instead of failing if free collateral is short)",
    )
    .with_default(false)
    .prompt()?;

    let confirm = Confirm::new(&format!(
        "Request withdrawal of {amount} {} to {recipient}?",
        collateral.symbol
    ))
    .with_default(true)
    .prompt()?;
    if !confirm {
        return Ok(());
    }

    println!("Requesting withdrawal...");
    // Ticket expiration is left at the contract default (None).
    let (nonce, tx_digest) = accounts
        .request_withdrawal(
            &details.account_id,
            &details.account_object_id,
            &details.manager_cap_id,
            raw,
            &recipient,
            None,
            best_effort,
        )
        .await
        .context("withdrawal request failed")?;

    println!("\n✓ Withdrawal requested");
    println!("  amount:      {amount} {}", collateral.symbol);
    println!("  recipient:   {recipient}");
    println!("  nonce:       {nonce}");
    println!("  transaction: {tx_digest}");
    return Ok(());
}

/// Scales a human-readable `amount` of the collateral asset to its raw
/// smallest-unit `u64` representation. Returns an error message suffix when the
/// amount has finer precision than the coin supports or overflows `u64`.
fn to_base_units(amount: Decimal, decimals: u8) -> Result<u64, String> {
    let scale = Decimal::from(10u64.pow(decimals as u32));
    let raw = amount * scale;
    if !raw.fract().is_zero() {
        return Err(format!("supports at most {decimals} decimal places."));
    }
    return raw
        .to_u64()
        .ok_or_else(|| "amount is too large.".to_string());
}

/// Formats a raw smallest-unit amount as a human-readable decimal string,
/// prefixing `-` when the balance is negative (a debt).
fn fmt_amount(value: u64, is_neg: bool, decimals: u8) -> String {
    let scale = Decimal::from(10u64.pow(decimals as u32));
    let amount = (Decimal::from(value) / scale).normalize();
    let sign = if is_neg && value != 0 { "-" } else { "" };
    return format!("{sign}{amount}");
}

#[derive(Clone, Copy)]
enum Action {
    Create,
    Deposit,
    Withdraw,
    Refresh,
    Quit,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Action::Create => "Create account",
            Action::Deposit => "Deposit funds",
            Action::Withdraw => "Withdraw funds",
            Action::Refresh => "Refresh",
            Action::Quit => "Quit",
        };
        return write!(f, "{label}");
    }
}

#[derive(Clone, Copy)]
enum DepositMethod {
    Wallet,
    VipFaucet,
}

impl fmt::Display for DepositMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            DepositMethod::Wallet => "From wallet (regular)",
            DepositMethod::VipFaucet => "VIP faucet mint",
        };
        return write!(f, "{label}");
    }
}

/// Runs `fut` to completion while animating a single-line braille spinner with
/// `message`. The spinner line is cleared once the future resolves. `biased`
/// select polls the work first, so a fast future returns without flicker.
async fn with_spinner<F, T>(message: &str, fut: F) -> T
where
    F: Future<Output = T>,
{
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    tokio::pin!(fut);
    let mut ticker = tokio::time::interval(Duration::from_millis(80));
    let mut frame = 0usize;
    loop {
        tokio::select! {
            biased;
            output = &mut fut => {
                print!("\r\x1b[2K");
                let _ = std::io::stdout().flush();
                return output;
            }
            _ = ticker.tick() => {
                print!("\r{} {message}", FRAMES[frame % FRAMES.len()]);
                let _ = std::io::stdout().flush();
                frame += 1;
            }
        }
    }
}
