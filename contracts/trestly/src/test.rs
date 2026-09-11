#![cfg(test)]

use super::{TrestlyContract, TrestlyContractClient};
use crate::types::ContractError;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (token::StellarAssetClient<'a>, token::Client<'a>) {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::StellarAssetClient::new(env, &contract_address.address()),
        token::Client::new(env, &contract_address.address()),
    )
}

fn setup_test_env() -> (
    Env,
    TrestlyContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TrestlyContract, ());
    let client = TrestlyContractClient::new(&env, &contract_id);

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_admin_client, token_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&payer, &1000);

    (env, client, payer, payee, arbiter, token_admin, token_client)
}

#[test]
fn test_create_payment_valid() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    assert_eq!(payment_id, 1);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.payer, payer);
    assert_eq!(payment.payee, payee);
    assert_eq!(payment.arbiter, arbiter);
    assert_eq!(payment.amount, 100);
    assert_eq!(payment.disputed, false);
    assert_eq!(payment.resolved, false);

    // Check contract holds the tokens
    assert_eq!(token.balance(&client.address), 100);
}

#[test]
fn test_create_payment_invalid_amount() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let result = client.try_create_payment(&payer, &payee, &token.address, &0, &1000, &arbiter);

    assert_eq!(result.err(), Some(Ok(ContractError::InvalidAmount.into())));
}

#[test]
fn test_raise_dispute_before_window_closes() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.disputed, true);
}

#[test]
fn test_raise_dispute_after_window_closes() {
    let (env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    // Advance time beyond dispute window
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 1001,
        protocol_version: 27,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let result = client.try_raise_dispute(&payment_id);

    assert_eq!(result.err(), Some(Ok(ContractError::DisputeWindowClosed.into())));
}

#[test]
fn test_raise_dispute_twice() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    let result = client.try_raise_dispute(&payment_id);

    assert_eq!(result.err(), Some(Ok(ContractError::AlreadyDisputed.into())));
}

#[test]
fn test_release_after_window_undisputed() {
    let (env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    // Advance time beyond dispute window
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 1001,
        protocol_version: 27,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    client.release(&payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.resolved, true);

    // Check payee received the tokens
    assert_eq!(token.balance(&payee), 100);
}

#[test]
fn test_release_before_window_closes() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    let result = client.try_release(&payment_id);

    assert_eq!(result.err(), Some(Ok(ContractError::DisputeWindowOpen.into())));
}

#[test]
fn test_release_disputed_payment() {
    let (env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    // Advance time beyond dispute window
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 1001,
        protocol_version: 27,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let result = client.try_release(&payment_id);

    assert_eq!(result.err(), Some(Ok(ContractError::AlreadyDisputed.into())));
}

#[test]
fn test_resolve_dispute_refund_to_payer() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    client.resolve_dispute(&payment_id, &true);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.resolved, true);

    // Check payer received refund (900 initial - 100 paid + 100 refund = 900)
    assert_eq!(token.balance(&payer), 1000);
}

#[test]
fn test_resolve_dispute_release_to_payee() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    client.resolve_dispute(&payment_id, &false);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.resolved, true);

    // Check payee received the tokens
    assert_eq!(token.balance(&payee), 100);
}

#[test]
fn test_resolve_already_resolved_payment() {
    let (_env, client, payer, payee, arbiter, _token_admin, token) = setup_test_env();

    let payment_id = client.create_payment(&payer, &payee, &token.address, &100, &1000, &arbiter);

    client.raise_dispute(&payment_id);

    client.resolve_dispute(&payment_id, &true);

    let result = client.try_resolve_dispute(&payment_id, &false);

    assert_eq!(result.err(), Some(Ok(ContractError::AlreadyResolved.into())));
}

#[test]
fn test_get_payment_nonexistent() {
    let (_env, client, _payer, _payee, _arbiter, _token_admin, _token) = setup_test_env();

    let result = client.try_get_payment(&999);

    assert_eq!(result.err(), Some(Ok(ContractError::PaymentNotFound.into())));
}
