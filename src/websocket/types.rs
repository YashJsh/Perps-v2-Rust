use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkPriceUpdate {
    pub s: String, // Symbol
    pub p: String, // Mark Price
    pub r: String, // Funding Rate
    pub E: String, // Event Time
}
