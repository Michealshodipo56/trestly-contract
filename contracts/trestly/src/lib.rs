#![no_std]

mod events;
mod storage;
mod test;
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
        token: Address,
        amount: i128,
        dispute_window_secs: u64,
        arbiter: Address,
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

    pub fn release(env: Env, payment_id: u32) -> Result<(), ContractError> {
        let mut payment = storage::get_payment(&env, payment_id)?;

        if payment.resolved {
            return Err(ContractError::AlreadyResolved);
        }

        if env.ledger().timestamp() < payment.dispute_window_end {
            return Err(ContractError::DisputeWindowOpen);
        }

        if payment.disputed {
            return Err(ContractError::AlreadyDisputed);
        }

        // Transfer tokens from contract to payee
        token::Client::new(&env, &payment.token).transfer(
            &env.current_contract_address(),
            &payment.payee,
            &payment.amount,
        );

        payment.resolved = true;
        storage::set_payment(&env, payment_id, &payment);

        events::released(&env, payment_id, &payment.payee, payment.amount);

        Ok(())
    }

    pub fn resolve_dispute(
        env: Env,
        payment_id: u32,
        refund_to_payer: bool,
    ) -> Result<(), ContractError> {
        let mut payment = storage::get_payment(&env, payment_id)?;

        payment.arbiter.require_auth();

        if payment.resolved {
            return Err(ContractError::AlreadyResolved);
        }

        if !payment.disputed {
            return Err(ContractError::DisputeWindowOpen);
        }

        let recipient = if refund_to_payer {
            &payment.payer
        } else {
            &payment.payee
        };

        // Transfer tokens from contract to recipient
        token::Client::new(&env, &payment.token).transfer(
            &env.current_contract_address(),
            recipient,
            &payment.amount,
        );

        payment.resolved = true;
        storage::set_payment(&env, payment_id, &payment);

        events::dispute_resolved(&env, payment_id, refund_to_payer);

        Ok(())
    }

    pub fn get_payment(env: Env, payment_id: u32) -> Result<EscrowedPayment, ContractError> {
        storage::get_payment(&env, payment_id)
    }
}
