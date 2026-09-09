# rust-test

TODO:
- I'm using decimal, but for this exercise maybe scaled i64 should be good enough and faster;
- I created and I'm using `TransactionsRegistry` to keep track of the transactions, but it's unbounded, meaning it can grow indefinitely. In a real-world scenario, we would need to implement some kind of cleanup or archiving mechanism to prevent unbounded growth and potential memory issues.
- I'm using `cargo llvm-cov` to check the code coverage so it can help me to easily identify untested parts of the code and easily create new tests to cover them


## Assumptions and behavior choices

This implementation follows the spec and makes the following explicit choices:

### Account creation
- If a transaction references a client that does not exist yet, a new account record is created automatically.
- This applies before processing each transaction type.

### Duplicate transaction IDs
- Transaction IDs are treated as globally unique across all transaction types.
- Reusing a `tx` ID is ignored as `TransactionDuplicated`.

### Dispute scope
- Only **deposit** transactions are disputable.
- Given the spec's wording, disputing a non-deposit transaction is ignored as `NotDisputableType`.
- A transaction can only be disputed once, otherwise promotes abuse and is ignored as `TransactionAlreadyDisputed`.

### Locked (frozen) account behavior
- After a successful chargeback, the account is locked (`is_locked = true`).
- On locked accounts:
  - `deposit` and `withdrawal` are ignored (`AccountLocked`).
  - `dispute`, `resolve`, and `chargeback` are still processed when otherwise valid.
- Rationale: lock blocks normal account movement, while dispute lifecycle operations may still complete for already-recorded transactions.

### Ignored vs error
- Business-invalid operations return `TransactionOutcome::Ignored(...)` (e.g., insufficient funds, tx not found, wrong client, not disputed).
- Hard failures (unexpected invariant breaks) return errors.
