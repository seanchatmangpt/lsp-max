use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosurePacket {
    pub status: String,
    pub f_t: usize,
    pub w_t: usize,
    pub c_t: usize,
    pub r_b: Vec<String>,
}
