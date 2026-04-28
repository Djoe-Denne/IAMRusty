use serde::{Deserialize, Serialize};

/// Pagination request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequest {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

impl PaginationRequest {
    pub fn page_size(&self) -> u32 {
        self.limit.unwrap_or(20).min(100)
    }

    pub fn page_size_with_defaults(&self, default_page_size: u32, max_page_size: u32) -> u32 {
        self.limit
            .unwrap_or(default_page_size)
            .min(max_page_size.max(1))
    }

    pub fn page(&self) -> u32 {
        self.cursor
            .as_ref()
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(0)
    }
}

/// Pagination response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationResponse {
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: Option<i64>,
}

impl PaginationResponse {
    pub fn new(next_cursor: Option<String>, has_more: bool, total_count: Option<i64>) -> Self {
        Self {
            next_cursor,
            has_more,
            total_count,
        }
    }
}
