use rust_decimal::Decimal;

pub struct Account {
    pub available: Decimal,
    pub held: Decimal,
    pub is_locked: bool,
}
impl Account {
    pub fn new() -> Self {
        Self { available: Decimal::ZERO, held: Decimal::ZERO, is_locked: false }
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
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
    fn new_creates_unlocked_account_with_zero_balances() {
        let account = Account::new();

        assert_eq!(account.available, Decimal::ZERO);
        assert_eq!(account.held, Decimal::ZERO);
        assert!(!account.is_locked);
        assert_eq!(account.total(), Decimal::ZERO);
    }

    #[test]
    fn total_is_available_plus_held() {
        let account = Account { available: d("10.2500"), held: d("1.7500"), is_locked: false };

        assert_eq!(account.total(), d("12.0000"));
    }

    #[test]
    fn total_handles_zero_and_negative_values() {
        let account = Account { available: d("-2.5000"), held: d("2.5000"), is_locked: true };

        assert_eq!(account.total(), Decimal::ZERO);
    }
}
