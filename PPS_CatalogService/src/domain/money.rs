use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(default = "usd_currency")]
    pub currency: String,
}

fn usd_currency() -> String {
    "USD".to_owned()
}

impl Money {
    pub fn validate(&self) -> Result<(), String> {
        if self.amount.is_sign_negative() {
            return Err("price amount cannot be negative".into());
        }
        if !self.currency.eq_ignore_ascii_case("USD") {
            return Err("only USD is supported".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_rejects_negative_amounts() {
        assert!(Money {
            amount: Decimal::new(-1, 0),
            currency: "USD".into()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn money_defaults_to_usd_and_rejects_other_currencies() {
        let usd: Money = serde_json::from_str(r#"{"amount":"10.00"}"#).unwrap();
        assert_eq!(usd.currency, "USD");
        assert!(usd.validate().is_ok());

        let eur: Money = serde_json::from_str(r#"{"amount":"10.00","currency":"EUR"}"#).unwrap();
        assert!(eur.validate().is_err());
    }
}
