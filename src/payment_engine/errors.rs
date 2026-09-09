use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Unexpected: account missing after `ensure_account()`")]
    MissingAccountAfterEnsure,

    #[error("Deposit overflow")]
    DepositOverflow,

    #[error("Withdrawal underflow")]
    WithdrawalUnderflow,
}
