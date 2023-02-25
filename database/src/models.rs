use sqlx::types::BigDecimal;

#[derive(sqlx::Type)]
#[sqlx(type_name = "cycle")]
pub struct Cycle {
    pub id: BigDecimal,
    pub block_number: BigDecimal,
    pub creator: String,
    pub starting_block: BigDecimal,
    pub block_length: BigDecimal,
    pub vote_price: BigDecimal,
    pub balance: BigDecimal,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "vote")]
pub struct Vote {
    pub id: BigDecimal,
    pub block_number: BigDecimal,
    pub cycle_id: BigDecimal,
    pub placer: String,
    pub symbol: [u8; 4],
    pub amount: BigDecimal,
    pub placement: BigDecimal,
}
