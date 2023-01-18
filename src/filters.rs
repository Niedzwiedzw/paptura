use rust_decimal::Decimal;

pub fn format_currency(money: Decimal) -> askama::Result<String> {
    Ok(format!("{:.2}", money))
}

pub fn format_percent(value: Decimal) -> askama::Result<String> {
    Ok(format!(
        "{:.2}%",
        value
            .checked_mul(Decimal::new(100, 0))
            .ok_or(askama::Error::Fmt(std::fmt::Error))?
    ))
}
