use anyhow::anyhow;
use rust_decimal::Decimal;
use serde::Deserialize;

use super::types::{ClientId, TransactionId, TransactionRecord, TransactionType};

#[derive(Debug, Deserialize)]
pub struct CsvTransactionRecord {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(rename = "tx")]
    transaction_id: TransactionId,
    amount: Option<Decimal>,
}
impl TryFrom<CsvTransactionRecord> for TransactionRecord {
    type Error = anyhow::Error;

    fn try_from(r: CsvTransactionRecord) -> Result<Self, Self::Error> {
        let CsvTransactionRecord { transaction_type, client_id, transaction_id, amount } = r;

        match transaction_type {
            TransactionType::Deposit => Ok(TransactionRecord::Deposit {
                client_id,
                transaction_id,
                amount: amount.ok_or_else(|| anyhow!("deposit requires amount"))?,
            }),
            TransactionType::Withdrawal => Ok(TransactionRecord::Withdrawal {
                client_id,
                transaction_id,
                amount: amount.ok_or_else(|| anyhow!("withdrawal requires amount"))?,
            }),
            TransactionType::Dispute => Ok(TransactionRecord::Dispute { client_id, transaction_id }),
            TransactionType::Resolve => Ok(TransactionRecord::Resolve { client_id, transaction_id }),
            TransactionType::Chargeback => Ok(TransactionRecord::Chargeback { client_id, transaction_id }),
        }
    }
}
