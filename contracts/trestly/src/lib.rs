#![no_std]

mod events;
mod storage;
mod types;

use soroban_sdk::{contract, contractimpl, token, Address, Env};
use types::{ContractError, EscrowedPayment};

#[contract]
pub struct TrestlyContract;

#[contractimpl]
impl TrestlyContract {
    pub fn create_payment(
        env: Env,
        payer: Address,
        payee: Address,
        arbiter: Address,
        token: Address,
        amount: i128,
        dispute_window_secs: u64,
    ) -> Result<u32, ContractError> {
        payer.require_auth();

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        // Transfer tokens from payer to contract
        token::Client::new(&env, &token).transfer(&payer, &env.current_contract_address(), &amount);

        // Increment payment count
        let payment_count = storage::get_payment_count(&env);
        let payment_id = payment_count + 1;
        storage::set_payment_count(&env, payment_id);

        // Create escrowed payment
        let dispute_window_end = env.ledger().timestamp() + dispute_window_secs;
        let payment = EscrowedPayment {
            payer: payer.clone(),
            payee: payee.clone(),
            token,
            amount,
            dispute_window_end,
            disputed: false,
            resolved: false,
            arbiter,
        };

        storage::set_payment(&env, payment_id, &payment);

        events::payment_created(&env, payment_id, &payer, &payee, amount);

        Ok(payment_id)
    }

    pub fn raise_dispute(env: Env, payment_id: u32) -> Result<(), ContractError> {
        let mut payment = storage::get_payment(&env, payment_id)?;

        payment.payer.require_auth();

        if payment.resolved {
            return Err(ContractError::AlreadyResolved);
        }

        if env.ledger().timestamp() >= payment.dispute_window_end {
            return Err(ContractError::DisputeWindowClosed);
        }

        if payment.disputed {
            return Err(ContractError::AlreadyDisputed);
        }

        payment.disputed = true;
        storage::set_payment(&env, payment_id, &payment);

        events::dispute_raised(&env, payment_id);

        Ok(())
    }
}
