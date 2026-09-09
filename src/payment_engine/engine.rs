use anyhow::Result;
use rust_decimal::Decimal;
use rustc_hash::FxHashMap;

use super::account::Account;
use super::errors::EngineError;
use super::transactions_registry::{TransactionType as RegistryTxType, TransactionsRegistry};
use super::types::{ClientId, IgnoreReason, TransactionId, TransactionOutcome, TransactionRecord};

pub struct PaymentsEngine {
    accounts: FxHashMap<ClientId, Account>,
    transactions_registry: TransactionsRegistry,
}
impl PaymentsEngine {
    pub fn new() -> Self {
        Self { accounts: FxHashMap::default(), transactions_registry: TransactionsRegistry::new() }
    }

    pub fn process_transaction(&mut self, record: TransactionRecord) -> Result<TransactionOutcome> {
        // ensure the account exists before processing the transaction
        let client_id = match &record {
            TransactionRecord::Deposit { client_id, .. }
            | TransactionRecord::Withdrawal { client_id, .. }
            | TransactionRecord::Dispute { client_id, .. }
            | TransactionRecord::Resolve { client_id, .. }
            | TransactionRecord::Chargeback { client_id, .. } => *client_id,
        };
        self.ensure_account(client_id);

        // process the transaction based on its type
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

    fn deposit(&mut self, client_id: ClientId, tx_id: TransactionId, amount: Decimal) -> Result<TransactionOutcome> {
        // ignore non-positive deposits
        if amount <= Decimal::ZERO {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
        }

        let account = self.accounts.get_mut(&client_id).ok_or(EngineError::MissingAccountAfterEnsure)?;

        // don't allow deposits to locked accounts
        if account.is_locked {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::AccountLocked));
        }

        // register the transaction in the registry (ignore duplicated transactions)
        let inserted = self.transactions_registry.insert(RegistryTxType::Deposit, tx_id, client_id, amount);
        if !inserted {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionDuplicated));
        }

        let new_amount = account.available.checked_add(amount).ok_or(EngineError::DepositOverflow)?;
        account.available = new_amount;

        Ok(TransactionOutcome::Applied)
    }

    fn withdrawal(&mut self, client_id: ClientId, tx_id: TransactionId, amount: Decimal) -> Result<TransactionOutcome> {
        // ignore non-positive deposits
        if amount <= Decimal::ZERO {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
        }

        let account = self.accounts.get_mut(&client_id).ok_or(EngineError::MissingAccountAfterEnsure)?;

        // don't allow withdrawals from locked accounts
        if account.is_locked {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::AccountLocked));
        }

        // no sufficient funds to withdraw
        if account.available < amount {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::InsufficientFunds));
        }

        // register the transaction in the registry (ignore duplicated transactions)
        let inserted = self.transactions_registry.insert(RegistryTxType::Withdrawal, tx_id, client_id, amount);
        if !inserted {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionDuplicated));
        }

        let new_amount = account.available.checked_sub(amount).ok_or(EngineError::WithdrawalUnderflow)?;
        account.available = new_amount;

        Ok(TransactionOutcome::Applied)
    }

    fn dispute(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // TODO: implement
        println!("Dispute: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(TransactionOutcome::Applied)
    }

    fn resolve(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // TODO: implement
        println!("Resolve: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(TransactionOutcome::Applied)
    }

    fn chargeback(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // TODO: implement
        println!("Chargeback: client_id={:?}, tx_id={:?}", client_id, tx_id);
        Ok(TransactionOutcome::Applied)
    }

    fn ensure_account(&mut self, client_id: ClientId) {
        self.accounts.entry(client_id).or_insert_with(Account::new);
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
    fn deposit_applies_for_valid_amount() {
        let mut engine = PaymentsEngine::new();

        let outcome = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("1.5000") })
            .unwrap();

        assert_eq!(outcome, TransactionOutcome::Applied);
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("1.5000"));
    }

    #[test]
    fn deposit_ignores_non_positive_amount() {
        let mut engine = PaymentsEngine::new();

        let zero = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: Decimal::ZERO })
            .unwrap();

        let negative = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 2, amount: d("-1.0") })
            .unwrap();

        assert_eq!(zero, TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
        assert_eq!(negative, TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
        assert_eq!(engine.accounts.get(&1).unwrap().available, Decimal::ZERO);
    }

    #[test]
    fn deposit_ignores_when_account_is_locked() {
        let mut engine = PaymentsEngine::new();
        engine.ensure_account(1);
        engine.accounts.get_mut(&1).unwrap().is_locked = true;

        let outcome = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("10.0") })
            .unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::AccountLocked));
        assert_eq!(engine.accounts.get(&1).unwrap().available, Decimal::ZERO);
    }

    #[test]
    fn deposit_ignores_duplicate_transaction_id() {
        let mut engine = PaymentsEngine::new();

        let first = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 100, amount: d("2.0") })
            .unwrap();

        let duplicate = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 100, amount: d("3.0") })
            .unwrap();

        assert_eq!(first, TransactionOutcome::Applied);
        assert_eq!(duplicate, TransactionOutcome::Ignored(IgnoreReason::TransactionDuplicated));
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("2.0"));
    }

    #[test]
    fn deposit_returns_error_on_overflow() {
        let mut engine = PaymentsEngine::new();
        engine.ensure_account(1);
        engine.accounts.get_mut(&1).unwrap().available = Decimal::MAX;

        let err = engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: Decimal::ONE })
            .unwrap_err();

        let engine_err = err.downcast_ref::<EngineError>().unwrap();
        assert_eq!(*engine_err, EngineError::DepositOverflow);
    }

    #[test]
    fn withdrawal_applies_for_valid_amount() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("5.0") })
            .unwrap();

        let outcome = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("1.5") })
            .unwrap();

        assert_eq!(outcome, TransactionOutcome::Applied);
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("3.5"));
    }

    #[test]
    fn withdrawal_ignores_non_positive_amount() {
        let mut engine = PaymentsEngine::new();

        let zero = engine
            .process_transaction(TransactionRecord::Withdrawal {
                client_id: 1,
                transaction_id: 1,
                amount: Decimal::ZERO,
            })
            .unwrap();

        let negative = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("-1.0") })
            .unwrap();

        assert_eq!(zero, TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
        assert_eq!(negative, TransactionOutcome::Ignored(IgnoreReason::NonPositiveAmount));
    }

    #[test]
    fn withdrawal_ignores_when_account_is_locked() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("5.0") })
            .unwrap();

        engine.accounts.get_mut(&1).unwrap().is_locked = true;

        let outcome = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("1.0") })
            .unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::AccountLocked));
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("5.0"));
    }

    #[test]
    fn withdrawal_ignores_insufficient_funds() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("1.0") })
            .unwrap();

        let outcome = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("2.0") })
            .unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::InsufficientFunds));
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("1.0"));
    }

    #[test]
    fn withdrawal_ignores_duplicate_transaction_id() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("10.0") })
            .unwrap();

        let first = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("1.0") })
            .unwrap();

        let duplicate = engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("1.0") })
            .unwrap();

        assert_eq!(first, TransactionOutcome::Applied);
        assert_eq!(duplicate, TransactionOutcome::Ignored(IgnoreReason::TransactionDuplicated));
        assert_eq!(engine.accounts.get(&1).unwrap().available, d("9.0"));
    }
}
