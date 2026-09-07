use crate::types::{ContractError, DataKey, EscrowedPayment};
use soroban_sdk::Env;

const LEDGER_EXTEND_AMOUNT: u32 = 50000;

pub fn get_payment_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::PaymentCount)
        .unwrap_or(0)
}

pub fn set_payment_count(env: &Env, count: u32) {
    env.storage().instance().set(&DataKey::PaymentCount, &count);
}

pub fn get_payment(env: &Env, payment_id: u32) -> Result<EscrowedPayment, ContractError> {
    let key = DataKey::Payment(payment_id);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::PaymentNotFound)
}

pub fn set_payment(env: &Env, payment_id: u32, payment: &EscrowedPayment) {
    let key = DataKey::Payment(payment_id);
    env.storage().persistent().set(&key, payment);
    env.storage()
        .persistent()
        .extend_ttl(&key, 0, LEDGER_EXTEND_AMOUNT);
}
