use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub response_time: Duration,
    pub links_found: Vec<String>,
    pub forms_found: usize,
    pub scripts_found: usize,
    pub error: Option<String>,
}

impl CrawlResult {
    pub fn new(url: String) -> Self {
        Self {
            url,
            status_code: 0,
            content_type: None,
            content_length: None,
            response_time: Duration::from_secs(0),
            links_found: Vec::new(),
            forms_found: 0,
            scripts_found: 0,
            error: None,
        }
    }

    pub fn with_error(url: String, error: String) -> Self {
        Self {
            url,
            status_code: 0,
            content_type: None,
            content_length: None,
            response_time: Duration::from_secs(0),
            links_found: Vec::new(),
            forms_found: 0,
            scripts_found: 0,
            error: Some(error),
        }
    }
}
