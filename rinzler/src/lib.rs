// Include handlers module directly from handlers.rs
#[path = "handlers.rs"]
pub mod handlers;

// Re-export commonly used handler functions for convenience
pub use handlers::{
    load_urls_from_file,
    load_urls_from_source,
    parse_url_line,
};

// Re-export crawl functionality from rinzler-core
pub use rinzler_core::crawl::{
    execute_crawl, extract_url_path, generate_crawl_report,
    CrawlOptions, CrawlProgressCallback, FollowMode,
};
