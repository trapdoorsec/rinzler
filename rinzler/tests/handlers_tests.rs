use rinzler::handlers::*;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use url::Url;

#[test]
fn test_parse_url_line_with_scheme() {
    let result = parse_url_line("https://example.com");
    assert_eq!(result, Some("https://example.com".to_string()));
}

#[test]
fn test_parse_url_line_without_scheme() {
    let result = parse_url_line("example.com");
    assert_eq!(result, Some("http://example.com".to_string()));
}

#[test]
fn test_parse_url_line_invalid() {
    let result = parse_url_line("not a valid url!!!");
    assert_eq!(result, None);
}

#[test]
fn test_extract_url_path() {
    assert_eq!(
        extract_url_path("https://example.com/api/users"),
        "/api/users"
    );
    assert_eq!(extract_url_path("https://example.com/"), "/");
    assert_eq!(extract_url_path("https://example.com"), "/");
}

#[test]
fn test_load_urls_from_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut temp_file = NamedTempFile::new()?;
    writeln!(temp_file, "https://example.com")?;
    writeln!(temp_file, "httpbin.org")?;
    writeln!(temp_file)?; // Empty line
    writeln!(temp_file, "https://api.example.com")?;

    let path = PathBuf::from(temp_file.path());
    let urls = load_urls_from_file(&path)?;

    assert_eq!(urls.len(), 3);
    assert_eq!(urls[0], "https://example.com");
    assert_eq!(urls[1], "http://httpbin.org");
    assert_eq!(urls[2], "https://api.example.com");

    Ok(())
}

#[test]
fn test_load_urls_from_file_empty() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "   ").unwrap();

    let path = PathBuf::from(temp_file.path());
    let result = load_urls_from_file(&path);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No valid URLs"));
}

#[test]
fn test_load_urls_from_source_single_url() {
    let url = Url::parse("https://example.com").unwrap();
    let result = load_urls_from_source(Some(&url), None).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "https://example.com/");
}

#[test]
fn test_load_urls_from_source_no_input() {
    let result = load_urls_from_source(None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Either --url or --hosts-file must be provided")
    );
}

#[test]
fn test_generate_crawl_report() {
    use rinzler_scanner::result::CrawlResult;
    use std::time::Duration;

    let results = vec![
        CrawlResult {
            url: "https://example.com/".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            content_length: Some(1024),
            response_time: Duration::from_millis(100),
            links_found: vec!["https://example.com/about".to_string()],
            forms_found: 1,
            scripts_found: 2,
            error: None,
        },
        CrawlResult {
            url: "https://example.com/api/data".to_string(),
            status_code: 200,
            content_type: Some("application/json".to_string()),
            content_length: Some(512),
            response_time: Duration::from_millis(50),
            links_found: vec![],
            forms_found: 0,
            scripts_found: 0,
            error: None,
        },
    ];

    let report = generate_crawl_report(&results);

    assert!(report.contains("Pages crawled: 2"));
    assert!(report.contains("Total links found: 1"));
    assert!(report.contains("Total forms found: 1"));
    assert!(report.contains("Total scripts found: 2"));
    assert!(report.contains("example.com"));
    assert!(report.contains("/api/data"));
    assert!(report.contains("application/json"));
    assert!(!report.contains("text/html")); // Should be hidden
}
