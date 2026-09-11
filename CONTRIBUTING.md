# Contributing to trestly-contract

Thanks for looking at this. This is the Soroban smart contract at the core of
Trestly — a dispute-aware escrow layer for x402 payments on Stellar.

## Prerequisites

- Rust (stable) with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- No `stellar-cli` / `soroban-cli` install required for day-to-day development
  — only for manual deployment, and even that has a CLI-free alternative (see
  [`scripts/deploy-testnet.cjs`](scripts/deploy-testnet.cjs)).

## Building and testing

```bash
cargo test                                          # unit tests
cargo build --target wasm32v1-none --release         # release wasm
```

CI runs both of these on every push and pull request to `main` — a PR can't
merge unless they pass (see `.github/workflows/ci.yml`).

## Project layout

- `contracts/trestly/src/lib.rs` — contract entrypoints: `create_payment`,
  `raise_dispute`, `release`, `resolve_dispute`, `get_payment`
- `contracts/trestly/src/types.rs` — `EscrowedPayment`, `ContractError`
- `contracts/trestly/src/storage.rs` — ledger storage helpers
- `contracts/trestly/src/events.rs` — `#[contractevent]` definitions
- `contracts/trestly/src/test.rs` — unit tests (12 covering the full
  create/dispute/release/resolve lifecycle)
- `scripts/deploy-testnet.cjs` — deploys to testnet via `@stellar/stellar-sdk`
  directly over RPC, no `stellar-cli` needed
- `scripts/e2e-testnet.cjs` — full lifecycle proof against a live deployment,
  signing with generated keypairs instead of a browser wallet

## Making a change

1. Fork and branch off `main`.
2. Add or update tests in `test.rs` for any behavior change — PRs that change
   contract logic without test coverage will be asked to add it.
3. Run `cargo test` and the wasm build locally before opening a PR.
4. Open a PR against `main`. CI must pass and the PR needs one approving
   review before it can merge (branch protection is on).

## Reporting issues

Open a GitHub issue. If you're picking up an issue that's part of a
[Drips Wave](https://www.drips.network/wave/stellar) cycle, it'll be labeled
accordingly — read the acceptance criteria in the issue body before starting,
and feel free to ask clarifying questions on the issue itself.

## Security

This contract has not had a third-party security audit. If you find a
vulnerability, please open an issue describing it rather than exploiting it
against the live testnet deployment — see the Security section of the
[README](README.md) for the current threat-model notes (arbiter trust model,
dispute-window assumptions).
