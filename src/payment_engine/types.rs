use rust_decimal::Decimal;
use serde::Deserialize;

pub type ClientId = u16;
pub type TransactionId = u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionRecord {
    Deposit { client_id: ClientId, transaction_id: TransactionId, amount: Decimal },
    Withdrawal { client_id: ClientId, transaction_id: TransactionId, amount: Decimal },
    Dispute { client_id: ClientId, transaction_id: TransactionId },
    Resolve { client_id: ClientId, transaction_id: TransactionId },
    Chargeback { client_id: ClientId, transaction_id: TransactionId },
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionOutcome {
    Applied,
    Ignored(IgnoreReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreReason {
    NonPositiveAmount,
    InsufficientFunds,
}
