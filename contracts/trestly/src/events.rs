use soroban_sdk::{contractevent, Address, Env};

#[contractevent(topics = ["created"], data_format = "vec")]
pub struct PaymentCreated {
    #[topic]
    pub payment_id: u32,
    pub payer: Address,
    pub payee: Address,
    pub amount: i128,
}

#[contractevent(topics = ["disputed"], data_format = "vec")]
pub struct DisputeRaised {
    #[topic]
    pub payment_id: u32,
}

#[contractevent(topics = ["released"], data_format = "vec")]
pub struct Released {
    #[topic]
    pub payment_id: u32,
    pub payee: Address,
    pub amount: i128,
}

#[contractevent(topics = ["resolved"], data_format = "vec")]
pub struct DisputeResolved {
    #[topic]
    pub payment_id: u32,
    pub refund_to_payer: bool,
}

pub fn payment_created(env: &Env, payment_id: u32, payer: &Address, payee: &Address, amount: i128) {
    PaymentCreated {
        payment_id,
        payer: payer.clone(),
        payee: payee.clone(),
        amount,
    }
    .publish(env);
}

pub fn dispute_raised(env: &Env, payment_id: u32) {
    DisputeRaised { payment_id }.publish(env);
}

pub fn released(env: &Env, payment_id: u32, payee: &Address, amount: i128) {
    Released {
        payment_id,
        payee: payee.clone(),
        amount,
    }
    .publish(env);
}

pub fn dispute_resolved(env: &Env, payment_id: u32, refund_to_payer: bool) {
    DisputeResolved {
        payment_id,
        refund_to_payer,
    }
    .publish(env);
}
