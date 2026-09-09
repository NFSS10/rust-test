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
