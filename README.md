# rust-test

A small transaction engine that reads CSV input, processes account operations, and writes final account balances to stdout.


## Table of Contents

- [Design](#design)
  - [Separation of concerns](#separation-of-concerns)
  - [Core components](#core-components)
- [Assumptions and behavior choices](#assumptions-and-behavior-choices)
- [Correctness](#correctness)
- [Safety and robustness](#safety-and-robustness)
- [Efficiency notes](#efficiency-notes)
- [Known limitations / future improvements](#known-limitations--future-improvements)


## Design

### Separation of concerns
- CSV parsing is separated from engine logic.
- Parsed input is converted into domain transactions before processing.
- The engine is responsible only for business rules and account state transitions.

### Core components
- `engine.rs`: transaction processing rules (`deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`);
- `transactions_registry.rs`: stores applied transactions by `tx` for dispute lifecycle handling;
- `account.rs`: account state (`available`, `held`, `locked`) and `total()` computation.


## Assumptions and behavior choices

- Accounts are auto-created when a transaction references an unknown client;
- Transaction IDs are globally unique; duplicate `tx` values are ignored;
- Only **deposits** are disputable in this implementation;
- A transaction can only be disputed once in its lifecycle;
- After chargeback, the account is locked.
  - On locked accounts:
    - `deposit` and `withdrawal` are ignored (`AccountLocked`).
    - `dispute`, `resolve`, and `chargeback` are still processed when otherwise valid.
  - Rationale: lock blocks normal account movement, while dispute lifecycle operations may still complete for already-recorded transactions.
- Business-invalid operations return `TransactionOutcome::Ignored(...)`;
- Invariant/system failures return errors (`EngineError`).


## Correctness

- Unit tests cover the important business logic and edge cases;
- `cargo llvm-cov` was used to validate path coverage and identify untested branches;
- Coverage is near 100% for:
  - `engine.rs`
  - `transactions_registry.rs`
  - `account.rs`

<img width="1406" height="818" alt="imagem" src="https://github.com/user-attachments/assets/203219c2-40cc-4c67-98a8-cdd1670de29f" />


## Safety and robustness

- Checked arithmetic is used (`checked_add`, `checked_sub`) to avoid silent overflow/underflow;
- Edge cases are handled explicitly (insufficient funds, missing tx, wrong client, invalid dispute state, duplicates, locked account);
- Error handling distinguishes:
  - expected business rejections
  - unexpected engine/invariant errors


## Efficiency notes

- Uses `rust_decimal` to avoid floating-point precision issues;
- Uses `FxHashMap` for faster hashing given that the input is trusted;
- Streaming CSV processing avoids loading all rows into memory at once.


## Known limitations / future improvements

- `TransactionsRegistry` is currently unbounded and can grow indefinitely. A production version should add retention/archival (e.g., bounded cache + persistent store);
- A scaled-integer representation (e.g., fixed 4-decimal `i64`) could improve performance, but was not chosen here for readability and maintainability;
