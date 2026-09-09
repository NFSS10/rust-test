use anyhow::Result;
use rust_decimal::Decimal;
use rustc_hash::FxHashMap;

use super::account::Account;
use super::types::{ClientId, TransactionId, TransactionRecord};

pub struct PaymentsEngine {
    accounts: FxHashMap<ClientId, Account>,
}
impl PaymentsEngine {
    pub fn new() -> Self {
        Self { accounts: FxHashMap::default() }
    }

    pub fn process_transaction(&mut self, record: TransactionRecord) -> Result<()> {
        match record {
            TransactionRecord::Deposit { client_id, transaction_id, amount } => {
                self.deposit(client_id, transaction_id, amount)
            }
            TransactionRecord::Withdrawal { client_id, transaction_id, amount } => {
                self.withdrawal(client_id, transaction_id, amount)
            }
            TransactionRecord::Dispute { client_id, transaction_id } => self.dispute(client_id, transaction_id),
            TransactionRecord::Resolve { client_id, transaction_id } => self.resolve(client_id, transaction_id),
            TransactionRecord::Chargeback { client_id, transaction_id } => self.chargeback(client_id, transaction_id),
        }
    }

    fn deposit(&mut self, client_id: ClientId, tx_id: TransactionId, amount: Decimal) -> Result<()> {
        // TODO: implement
        println!("Deposit: client_id={:?}, tx_id={:?}, amount={:?}", client_id, tx_id, amount);
        Ok(())
    }

    fn withdrawal(&mut self, client_id: ClientId, tx_id: TransactionId, amount: Decimal) -> Result<()> {
        // TODO: implement
        println!("Withdrawal: client_id={:?}, tx_id={:?}, amount={:?}", client_id, tx_id, amount);
        Ok(())
    }

    fn dispute(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<()> {
        // TODO: implement
        println!("Dispute: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(())
    }

    fn resolve(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<()> {
        // TODO: implement
        println!("Resolve: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(())
    }

    fn chargeback(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<()> {
        // TODO: implement
        println!("Chargeback: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(())
    }
}
