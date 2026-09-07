use soroban_sdk::{contracttype, contracterror, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    PaymentCount,
    Payment(u32),
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    PaymentNotFound = 1,
    AlreadyResolved = 2,
    AlreadyDisputed = 3,
    DisputeWindowClosed = 4,
    DisputeWindowOpen = 5,
    Unauthorized = 6,
    InvalidAmount = 7,
}
