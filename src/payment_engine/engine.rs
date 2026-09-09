use anyhow::Result;
use rust_decimal::Decimal;
use rustc_hash::FxHashMap;

use super::account::Account;
use super::errors::EngineError;
use super::transactions_registry::{DisputeState, TransactionType as RegistryTxType, TransactionsRegistry};
use super::types::{AccountSnapshot, ClientId, IgnoreReason, TransactionId, TransactionOutcome, TransactionRecord};

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

    pub fn accounts_snapshot(&self) -> Vec<AccountSnapshot> {
        let mut snapshots: Vec<AccountSnapshot> = self
            .accounts
            .iter()
            .map(|(&client_id, account)| AccountSnapshot {
                client_id,
                available: account.available,
                held: account.held,
                total: account.total(),
                is_locked: account.is_locked,
            })
            .collect();

        // sort by client_id for consistent output
        snapshots.sort_by_key(|snapshot| snapshot.client_id);

        snapshots
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

        // register the transaction in the registry (ignore duplicated transactions)
        let inserted = self.transactions_registry.insert(RegistryTxType::Withdrawal, tx_id, client_id, amount);
        if !inserted {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionDuplicated));
        }

        // no sufficient funds to withdraw
        if account.available < amount {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::InsufficientFunds));
        }

        let new_amount = account.available.checked_sub(amount).ok_or(EngineError::WithdrawalUnderflow)?;
        account.available = new_amount;

        Ok(TransactionOutcome::Applied)
    }

    fn dispute(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // can't dispute a transaction that wasn't registered for this client
        let transaction = match self.transactions_registry.get_mut(tx_id) {
            Some(t) => t,
            None => return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound)),
        };

        if transaction.client_id != client_id {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::WrongClient));
        }

        // only allow disputes on transactions that are in a clean state
        // (not already disputed, resolved, or charged back)
        if transaction.state != DisputeState::None {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionAlreadyDisputed));
        }

        // ASSUMPTION: only deposits are disputable in this toy engine
        if transaction.transaction_type != RegistryTxType::Deposit {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::NotDisputableType));
        }

        let account = self.accounts.get_mut(&client_id).ok_or(EngineError::MissingAccountAfterEnsure)?;

        let new_available = account.available.checked_sub(transaction.amount).ok_or(EngineError::DisputeUnderflow)?;
        let new_held = account.held.checked_add(transaction.amount).ok_or(EngineError::DisputeOverflow)?;

        account.available = new_available;
        account.held = new_held;
        transaction.state = DisputeState::Disputed;

        Ok(TransactionOutcome::Applied)
    }

    fn resolve(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // can't resolve a transaction that wasn't registered for this client
        let transaction = match self.transactions_registry.get_mut(tx_id) {
            Some(t) => t,
            None => return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound)),
        };

        if transaction.client_id != client_id {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::WrongClient));
        }
        if transaction.state != DisputeState::Disputed {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionNotInDispute));
        }

        let account = self.accounts.get_mut(&client_id).ok_or(EngineError::MissingAccountAfterEnsure)?;

        let new_held = account.held.checked_sub(transaction.amount).ok_or(EngineError::DisputeUnderflow)?;
        let new_available = account.available.checked_add(transaction.amount).ok_or(EngineError::DisputeOverflow)?;

        account.held = new_held;
        account.available = new_available;
        transaction.state = DisputeState::None;

        Ok(TransactionOutcome::Applied)
    }

    fn chargeback(&mut self, client_id: ClientId, tx_id: TransactionId) -> Result<TransactionOutcome> {
        // can't chargeback a transaction that wasn't registered for this client
        let transaction = match self.transactions_registry.get_mut(tx_id) {
            Some(t) => t,
            None => return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound)),
        };

        if transaction.client_id != client_id {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::WrongClient));
        }
        if transaction.state != DisputeState::Disputed {
            return Ok(TransactionOutcome::Ignored(IgnoreReason::TransactionNotInDispute));
        }

        let account = self.accounts.get_mut(&client_id).ok_or(EngineError::MissingAccountAfterEnsure)?;

        let new_held = account.held.checked_sub(transaction.amount).ok_or(EngineError::DisputeUnderflow)?;

        account.is_locked = true;
        account.held = new_held;
        transaction.state = DisputeState::ChargedBack;

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
        assert_eq!(engine.accounts.get(&1).unwrap().total(), d("3.5"));
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

    #[test]
    fn dispute_ignores_transaction_not_found() {
        let mut engine = PaymentsEngine::new();

        let outcome =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 999 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound));
    }

    #[test]
    fn dispute_ignores_wrong_client() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 2, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::WrongClient));
    }

    #[test]
    fn dispute_ignores_already_disputed_transaction() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();

        let first = engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        let second =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        assert_eq!(first, TransactionOutcome::Applied);
        assert_eq!(second, TransactionOutcome::Ignored(IgnoreReason::TransactionAlreadyDisputed));
    }

    #[test]
    fn dispute_ignores_not_disputable_type() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("5.0") })
            .unwrap();

        engine
            .process_transaction(TransactionRecord::Withdrawal { client_id: 1, transaction_id: 2, amount: d("1.0") })
            .unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 2 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::NotDisputableType));
    }

    #[test]
    fn dispute_applies_and_moves_funds_to_held() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 10, amount: d("3.5") })
            .unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 10 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Applied);
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, d("0.0"));
        assert_eq!(account.held, d("3.5"));
        assert_eq!(account.total(), d("3.5"))
    }

    #[test]
    fn dispute_returns_error_on_overflow_in_held() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: Decimal::ONE })
            .unwrap();

        {
            let account = engine.accounts.get_mut(&1).unwrap();
            account.available = Decimal::ONE;
            account.held = Decimal::MAX;
        }

        let err =
            engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap_err();

        let engine_err = err.downcast_ref::<EngineError>().unwrap();
        assert_eq!(*engine_err, EngineError::DisputeOverflow);
    }

    #[test]
    fn resolve_ignores_transaction_not_found() {
        let mut engine = PaymentsEngine::new();

        let outcome =
            engine.process_transaction(TransactionRecord::Resolve { client_id: 1, transaction_id: 999 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound));
    }

    #[test]
    fn resolve_ignores_wrong_client() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();
        engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Resolve { client_id: 2, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::WrongClient));
    }

    #[test]
    fn resolve_ignores_transaction_not_in_dispute() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Resolve { client_id: 1, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::TransactionNotInDispute));
    }

    #[test]
    fn resolve_applies_and_releases_held_funds() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();
        engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Resolve { client_id: 1, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Applied);
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, d("2.0"));
        assert_eq!(account.held, d("0.0"));
        assert_eq!(account.total(), d("2.0"));
    }

    #[test]
    fn resolve_returns_error_on_overflow_in_available() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: Decimal::ONE })
            .unwrap();
        engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        {
            let account = engine.accounts.get_mut(&1).unwrap();
            account.held = Decimal::ONE;
            account.available = Decimal::MAX;
        }

        let err =
            engine.process_transaction(TransactionRecord::Resolve { client_id: 1, transaction_id: 1 }).unwrap_err();

        let engine_err = err.downcast_ref::<EngineError>().unwrap();
        assert_eq!(*engine_err, EngineError::DisputeOverflow);
    }

    #[test]
    fn chargeback_ignores_transaction_not_found() {
        let mut engine = PaymentsEngine::new();

        let outcome =
            engine.process_transaction(TransactionRecord::Chargeback { client_id: 1, transaction_id: 999 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::TransactionNotFound));
    }

    #[test]
    fn chargeback_ignores_wrong_client() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();
        engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Chargeback { client_id: 2, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::WrongClient));
    }

    #[test]
    fn chargeback_ignores_transaction_not_in_dispute() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Chargeback { client_id: 1, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Ignored(IgnoreReason::TransactionNotInDispute));
    }

    #[test]
    fn chargeback_applies_and_locks_account() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: d("2.0") })
            .unwrap();
        engine.process_transaction(TransactionRecord::Dispute { client_id: 1, transaction_id: 1 }).unwrap();

        let outcome =
            engine.process_transaction(TransactionRecord::Chargeback { client_id: 1, transaction_id: 1 }).unwrap();

        assert_eq!(outcome, TransactionOutcome::Applied);

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.held, d("0.0"));
        assert!(account.is_locked);
        assert_eq!(account.total(), d("0.0"));

        let tx = engine.transactions_registry.get(1).unwrap();
        assert_eq!(tx.state, DisputeState::ChargedBack);
    }

    #[test]
    fn chargeback_returns_error_on_held_underflow() {
        let mut engine = PaymentsEngine::new();

        engine
            .process_transaction(TransactionRecord::Deposit { client_id: 1, transaction_id: 1, amount: Decimal::ONE })
            .unwrap();

        {
            let tx = engine.transactions_registry.get_mut(1).unwrap();
            tx.state = DisputeState::Disputed;
        }
        {
            let account = engine.accounts.get_mut(&1).unwrap();
            account.held = Decimal::MIN;
        }

        let err =
            engine.process_transaction(TransactionRecord::Chargeback { client_id: 1, transaction_id: 1 }).unwrap_err();

        let engine_err = err.downcast_ref::<EngineError>().unwrap();
        assert_eq!(*engine_err, EngineError::DisputeUnderflow);
    }
}
