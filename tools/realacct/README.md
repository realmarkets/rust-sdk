# realacct

Interactive CLI to inspect and manage a single Real Markets trading account from
an IOTA private key.

On launch it fetches the latest contract artifacts from the selected network's
indexer, connects to the IOTA chain, derives your address from the private key,
and shows the account's on-chain status. It then offers the relevant next
action: create the account if none exists yet, or deposit funds if one does.

## Usage

```bash
cargo run -p realacct -- --private-key iotaprivkey1... [--network testnet] [--account-index 0]
```

### Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--private-key` | `-p` | _(required)_ | Your IOTA private key (`iotaprivkey1...`). |
| `--network` | `-n` | `testnet` | Network to operate on. Selects both the indexer and the IOTA chain. |
| `--account-index` | `-i` | `0` | Account index in the registry. |

### Networks

`--network` picks a matched indexer + chain pair:

| Network | Indexer | IOTA chain |
|---------|---------|------------|
| `testnet` | `https://indexer.api.testnet.real.xyz` | testnet |

## What you'll see

```
  address  0x1a2b…f9
  index    0
  asset    STABLE (6 decimals)
  account  no account created yet
  VIP      no

? Action:
> Create account
  Refresh
  Quit
```

Once an account exists, the header also shows its current cross-margin
collateral **balance** on the exchange, and the menu offers **Deposit funds**
and **Withdraw funds**:

```
  address  0x1a2b…f9
  index    0
  asset    STABLE (6 decimals)
  account  0xacc1…
  id       0x…
  balance  42.5 STABLE
  VIP      yes

? Action:
> Deposit funds
  Withdraw funds
  Refresh
  Quit
```

Deposits can be made **From wallet (regular)**, which splits coins from your own
balance, or via **VIP faucet mint**, which mints collateral without needing
wallet coins. The VIP option only appears when your address is on the faucet's
VIP sender list.

**Withdraw funds** requests a withdrawal ticket (`request_withdrawal`) for a
given amount to a recipient address (defaulting to your own). By default it is
strict — it fails if your *free* collateral is short; choose *best effort* to
withdraw whatever is available instead. On success it prints the withdrawal
**nonce** and transaction digest. (The ticket uses the contract's default
expiration; redeeming it is a separate step not handled by this tool.)

Amounts are entered in human-readable units of the collateral asset (whose
symbol and decimals are read from its on-chain `CoinMetadata`), and the tool
scales them to the chain's raw representation for you:

```
? Deposit method: From wallet (regular)
? Amount to deposit (STABLE): 5.25
Depositing 5.25 STABLE (5250000 base units) from wallet...
```

Note: the VIP faucet mints whole-token amounts only (the faucet contract scales
by the coin's decimals on-chain), so fractional amounts are rejected for that
method.

## Notes

- **VIP status is address-scoped, not account-scoped.** It is a faucet sender
  allowlist keyed by the transaction sender, so the status reflects your key's
  address rather than a specific account index. Membership is read live from the
  on-chain faucet object on every refresh. A regular user cannot grant
  themselves VIP — that requires the faucet capability held by the deployer.
- **No `mainnet` option yet** — `--network` only accepts `testnet`.
