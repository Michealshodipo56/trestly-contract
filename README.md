# Trestly Contract

A Soroban smart contract for escrowing x402 agent micropayments on Stellar, providing a dispute and refund mechanism.

## Overview

Trestly extends the [x402 protocol](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/402) by adding an escrow layer for payments made between clients and servers. Instead of releasing funds immediately to the seller, Trestly holds payments in escrow with a configurable dispute window, allowing buyers to raise disputes before final settlement.

### Why Trestly?

The x402 protocol enables HTTP-native per-request payments using Soroban authorization entries. However, if a server accepts payment but fails to deliver the promised resource, the official spec provides no refund mechanism. Trestly fills this gap as a native Stellar refund layer (the only existing refund extension, x402r, is EVM-only).

## Features

- **Escrowed Payments**: Funds are held in the contract until the dispute window closes
- **Dispute Mechanism**: Payers can raise disputes during the dispute window
- **Arbiter Resolution**: Disputes are resolved by a designated arbiter who can refund to payer or release to payee
- **Automatic Release**: Undisputed payments are automatically releasable after the dispute window expires
- **No Instant Settlement**: All payments go through the escrow flow, preventing payment-without-delivery scenarios

## Architecture

### Contract Functions

#### `create_payment`
```rust
pub fn create_payment(
    env: Env,
    payer: Address,
    payee: Address,
    token: Address,
    amount: i128,
    dispute_window_secs: u64,
    arbiter: Address,
) -> Result<u32, ContractError>
```
Creates a new escrowed payment. Transfers tokens from payer to contract and returns a unique payment ID.

**Authorization**: Requires `payer` signature

**Errors**:
- `InvalidAmount`: If amount <= 0

#### `raise_dispute`
```rust
pub fn raise_dispute(env: Env, payment_id: u32) -> Result<(), ContractError>
```
Marks a payment as disputed. Only the payer can raise disputes, and only within the dispute window.

**Authorization**: Requires `payer` signature

**Errors**:
- `PaymentNotFound`: Payment ID doesn't exist
- `AlreadyResolved`: Payment already settled
- `DisputeWindowClosed`: Dispute window has expired
- `AlreadyDisputed`: Payment already disputed

#### `release`
```rust
pub fn release(env: Env, payment_id: u32) -> Result<(), ContractError>
```
Releases escrowed funds to the payee. Callable by anyone after the dispute window closes, but only for undisputed payments.

**Authorization**: None required (public function)

**Errors**:
- `PaymentNotFound`: Payment ID doesn't exist
- `AlreadyResolved`: Payment already settled
- `DisputeWindowOpen`: Dispute window still active
- `AlreadyDisputed`: Payment is disputed (must use `resolve_dispute`)

#### `resolve_dispute`
```rust
pub fn resolve_dispute(
    env: Env,
    payment_id: u32,
    refund_to_payer: bool,
) -> Result<(), ContractError>
```
Resolves a disputed payment. The arbiter decides whether to refund the payer or release to the payee.

**Authorization**: Requires `arbiter` signature

**Errors**:
- `PaymentNotFound`: Payment ID doesn't exist
- `AlreadyResolved`: Payment already settled
- `DisputeWindowOpen`: Payment not disputed (reused error variant)

#### `get_payment`
```rust
pub fn get_payment(env: Env, payment_id: u32) -> Result<EscrowedPayment, ContractError>
```
Read-only function to retrieve payment details.

**Errors**:
- `PaymentNotFound`: Payment ID doesn't exist

### Data Structures

```rust
pub struct EscrowedPayment {
    pub payer: Address,
    pub payee: Address,
    pub token: Address,
    pub amount: i128,
    pub dispute_window_end: u64,
    pub disputed: bool,
    pub resolved: bool,
    pub arbiter: Address,
}

pub enum ContractError {
    PaymentNotFound = 1,
    AlreadyResolved = 2,
    AlreadyDisputed = 3,
    DisputeWindowClosed = 4,
    DisputeWindowOpen = 5,
    Unauthorized = 6,
    InvalidAmount = 7,
}
```

### Events

The contract emits the following events:

- **PaymentCreated**: `{ payment_id: u32, payer: Address, payee: Address, amount: i128 }`
- **DisputeRaised**: `{ payment_id: u32 }`
- **Released**: `{ payment_id: u32, payee: Address, amount: i128 }`
- **DisputeResolved**: `{ payment_id: u32, refund_to_payer: bool }`

## Development

### Prerequisites

- Rust 2021 edition (1.84+ with wasm32v1-none target, or 1.81 or earlier with wasm32-unknown-unknown)
- Soroban CLI (latest stable)
- [soroban-sdk 27.0.6](https://docs.rs/soroban-sdk/27.0.6)

### Build

For Rust 1.84+:
```bash
rustup target add wasm32v1-none
cargo build --target wasm32v1-none --release
```

For Rust 1.81 or earlier:
```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM will be in `target/wasm32v1-none/release/trestly.wasm` (or `target/wasm32-unknown-unknown/release/trestly.wasm`).

### Test

```bash
cargo test
```

All tests are located in `contracts/trestly/src/test.rs` and cover:
- Valid payment creation
- Invalid amount rejection
- Dispute raising (before/after window)
- Duplicate dispute prevention
- Release mechanisms (before/after window, disputed/undisputed)
- Arbiter dispute resolution (refund vs. release)
- Payment retrieval

### Optimize

For production deployment, use the release profile with optimizations:

For Rust 1.84+:
```bash
cargo build --target wasm32v1-none --release --profile release
```

For Rust 1.81 or earlier:
```bash
cargo build --target wasm32-unknown-unknown --release --profile release
```

The `release` profile in `Cargo.toml` is configured for:
- Maximum size optimization (`opt-level = "z"`)
- Link-time optimization (LTO)
- Symbol stripping
- Single codegen unit

## Deployed Contract

**Testnet**: [`CBL4JVIPQBSTGUUVQRZXHDCGCVDPN3N4MHJKKY4MZSM2ZBILIIUJR6WD`](https://stellar.expert/explorer/testnet/contract/CBL4JVIPQBSTGUUVQRZXHDCGCVDPN3N4MHJKKY4MZSM2ZBILIIUJR6WD)

Verified end-to-end via `scripts/e2e-testnet.cjs` — both the happy path (create → wait → release) and the dispute path (create → raise_dispute → resolve_dispute) executed successfully through the actual `trestly-sdk` against this live instance.

## Deployment

### Testnet Deployment (no `stellar-cli` required)

`stellar-cli` is a heavy build (large dependency tree, memory-hungry link step) and isn't required to deploy — the same result can be reached with `@stellar/stellar-sdk` directly over RPC:

```bash
cd trestly-contract
cargo build --target wasm32v1-none --release
node scripts/deploy-testnet.cjs
```

This generates a throwaway deployer keypair, funds it via Friendbot, uploads the wasm, and creates the contract instance — printing the resulting contract ID and saving deployer details to `scripts/testnet-deployer.json` (gitignored). Requires Node.js and the dependencies already installed under `../trestly-sdk/node_modules` (run `npm install` there first if needed).

To re-run the full lifecycle proof against a deployment:
```bash
node scripts/e2e-testnet.cjs
```

### Testnet Deployment (via `stellar-cli`)

If you do have `stellar-cli` installed:

1. Configure testnet identity:
```bash
stellar keys generate deployer --network testnet
```

2. Fund the deployer account:
```bash
stellar keys fund deployer --network testnet
```

3. Build and deploy:
```bash
cd contracts/trestly
stellar contract deploy \
  --wasm ../../target/wasm32v1-none/release/trestly.wasm \
  --source deployer \
  --network testnet
```

4. Save the contract ID for interaction.

### Mainnet Deployment

⚠️ **Production deployment requires thorough security review and testing.**

Follow the same steps as testnet, but:
- Use `--network mainnet` instead of `--network testnet`
- Fund from a mainnet account
- Consider multi-signature deployment for critical contracts
- Audit all auth flows and economic parameters

## Usage Example

```rust
// Create payment with 24-hour dispute window
let payment_id = contract.create_payment(
    &buyer_address,
    &seller_address,
    &token_address,
    &1_000_000, // 1 token (assuming 7 decimals)
    &86400,     // 24 hours in seconds
    &arbiter_address,
);

// If service delivered correctly, wait 24 hours and release
contract.release(&payment_id);

// If service not delivered, buyer raises dispute
contract.raise_dispute(&payment_id);

// Arbiter investigates and refunds
contract.resolve_dispute(&payment_id, &true); // true = refund to payer
```

## Security Considerations

- **Arbiter Trust**: The arbiter has full control over disputed payments. Choose arbiters carefully.
- **Dispute Window**: Set appropriate dispute windows based on service delivery time expectations.
- **Token Approval**: Payers must approve token transfers before calling `create_payment`.
- **Storage TTL**: Payment data is extended for 50,000 ledgers (~7 days) on each write. Extend manually if needed.

## License

This contract is provided as-is for the Stellar community. Review and audit before production use.

## Links

- [Soroban Documentation](https://developers.stellar.org/docs/smart-contracts)
- [Stellar Laboratory](https://laboratory.stellar.org/)
- [x402 Protocol Spec](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/402)
