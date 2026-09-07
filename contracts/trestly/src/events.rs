use soroban_sdk::{symbol_short, Address, Env};

pub fn payment_created(env: &Env, payment_id: u32, payer: &Address, payee: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("created"),),
        (payment_id, payer, payee, amount),
    );
}

pub fn dispute_raised(env: &Env, payment_id: u32) {
    env.events().publish(
        (symbol_short!("disputed"),),
        payment_id,
    );
}

pub fn released(env: &Env, payment_id: u32, payee: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("released"),),
        (payment_id, payee, amount),
    );
}

pub fn dispute_resolved(env: &Env, payment_id: u32, refund_to_payer: bool) {
    env.events().publish(
        (symbol_short!("resolved"),),
        (payment_id, refund_to_payer),
    );
}
