use rust_decimal::Decimal;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

use super::types::{ClientId, TransactionId};

// TODO: Currently the registry is unbounded, which means it can hit memory issues if there are too many transactions. It needs a way to
// keep only the last N transactions or to persist them somewhere else (like disk or a database). For the sake of this exercise, I'm
// assuming that the number of transactions is small enough to fit in memory and that it's good enough for the purpose of this exercise.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeState {
    None,
    Disputed,
    Resolved,
    ChargedBack,
}

#[derive(Debug, Clone, Copy)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub client_id: ClientId,
    pub amount: Decimal,
    pub state: DisputeState,
}

pub struct TransactionsRegistry {
    transactions: FxHashMap<TransactionId, Transaction>,
}
impl TransactionsRegistry {
    pub fn new() -> Self {
        Self { transactions: FxHashMap::default() }
    }

    /// Inserts a new transaction into the registry.
    /// Returns true if the transaction was successfully inserted, false if a transaction with the same ID already exists.
    pub fn insert(
        &mut self,
        tx_type: TransactionType,
        tx_id: TransactionId,
        client_id: ClientId,
        amount: Decimal,
    ) -> bool {
        let tx = Transaction { transaction_type: tx_type, client_id, amount, state: DisputeState::None };

        let inserted = match self.transactions.entry(tx_id) {
            Entry::Vacant(slot) => {
                slot.insert(tx);
                true
            }
            Entry::Occupied(_) => false,
        };
        inserted
    }

    #[allow(dead_code)]
    pub fn get(&self, tx_id: TransactionId) -> Option<&Transaction> {
        self.transactions.get(&tx_id)
    }

    pub fn get_mut(&mut self, tx_id: TransactionId) -> Option<&mut Transaction> {
        self.transactions.get_mut(&tx_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(v: &str) -> Decimal {
        Decimal::from_str(v).unwrap()
    }

    #[test]
    fn insert_returns_true_for_new_tx_and_false_for_duplicate() {
        let mut registry = TransactionsRegistry::new();

        let first = registry.insert(TransactionType::Deposit, 1, 10, d("1.2500"));
        let second = registry.insert(TransactionType::Withdrawal, 1, 10, d("9.9999"));

        assert!(first);
        assert!(!second);

        let tx = registry.get(1).unwrap();
        assert_eq!(tx.client_id, 10);
        assert_eq!(tx.amount, d("1.2500"));
        assert_eq!(tx.transaction_type, TransactionType::Deposit);
        assert_eq!(tx.state, DisputeState::None);
    }

    #[test]
    fn get_returns_none_for_missing_tx() {
        let registry = TransactionsRegistry::new();
        assert!(registry.get(999).is_none());
    }

    #[test]
    fn get_returns_inserted_transaction() {
        let mut registry = TransactionsRegistry::new();
        registry.insert(TransactionType::Withdrawal, 42, 7, d("3.5000"));

        let tx = registry.get(42).unwrap();
        assert_eq!(tx.transaction_type, TransactionType::Withdrawal);
        assert_eq!(tx.client_id, 7);
        assert_eq!(tx.amount, d("3.5000"));
        assert_eq!(tx.state, DisputeState::None);
    }

    #[test]
    fn get_mut_allows_updating_dispute_state() {
        let mut registry = TransactionsRegistry::new();
        registry.insert(TransactionType::Deposit, 5, 1, d("2.0000"));

        let tx = registry.get_mut(5).unwrap();
        tx.state = DisputeState::Disputed;

        let tx = registry.get(5).unwrap();
        assert_eq!(tx.state, DisputeState::Disputed);
    }

    #[test]
    fn duplicate_insert_does_not_overwrite_existing_transaction() {
        let mut registry = TransactionsRegistry::new();
        registry.insert(TransactionType::Deposit, 77, 1, d("4.0000"));

        let inserted = registry.insert(TransactionType::Withdrawal, 77, 2, d("8.0000"));
        assert!(!inserted);

        let tx = registry.get(77).unwrap();
        assert_eq!(tx.transaction_type, TransactionType::Deposit);
        assert_eq!(tx.client_id, 1);
        assert_eq!(tx.amount, d("4.0000"));
        assert_eq!(tx.state, DisputeState::None);
    }
}
