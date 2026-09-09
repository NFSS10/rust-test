use anyhow::Result;
use rustc_hash::FxHashMap;

use super::account::Account;
use super::types::{ClientId, TransactionRecord};

pub struct PaymentsEngine {
    accounts: FxHashMap<ClientId, Account>,
}
impl PaymentsEngine {
    pub fn new() -> Self {
        Self { accounts: FxHashMap::default() }
    }

    pub fn process_transaction(&mut self, record: TransactionRecord) -> Result<()> {
        match record.transaction_type {
            super::types::TransactionType::Deposit => self.deposit(&record),
            super::types::TransactionType::Withdrawal => self.withdrawal(&record),
            super::types::TransactionType::Dispute => self.dispute(&record),
            super::types::TransactionType::Resolve => self.resolve(&record),
            super::types::TransactionType::Chargeback => self.chargeback(&record),
        }
    }

    fn deposit(&mut self, record: &TransactionRecord) -> Result<()> {
        todo!("Implement this");
    }

    fn withdrawal(&mut self, record: &TransactionRecord) -> Result<()> {
        todo!("Implement this");
    }

    fn dispute(&mut self, record: &TransactionRecord) -> Result<()> {
        todo!("Implement this");
    }

    fn resolve(&mut self, record: &TransactionRecord) -> Result<()> {
        todo!("Implement this");
    }

    fn chargeback(&mut self, record: &TransactionRecord) -> Result<()> {
        todo!("Implement this");
    }
}
