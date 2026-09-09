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
