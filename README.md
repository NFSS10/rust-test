# rust-test

TODO:
- I'm using decimal, but for this exercise maybe scaled i64 should be good enough and faster;
- I created and I'm using `TransactionsRegistry` to keep track of the transactions, but it's unbounded, meaning it can grow indefinitely. In a real-world scenario, we would need to implement some kind of cleanup or archiving mechanism to prevent unbounded growth and potential memory issues.
- I'm using `cargo llvm-cov` to check the code coverage so it can help me to easily identify untested parts of the code and easily create new tests to cover them
