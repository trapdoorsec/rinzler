// Tests for crawl functionality

use rinzler_core::crawl::{FollowMode, extract_url_path};

// ============================================================================
// URL Path Extraction Tests
// ============================================================================

#[test]
fn test_extract_url_path_root() {
    let url = "http://example.com/";
    let path = extract_url_path(url);
    assert_eq!(path, "/");
}

#[test]
fn test_extract_url_path_empty_path() {
    let url = "http://example.com";
    let path = extract_url_path(url);
    assert_eq!(path, "/");
}

#[test]
fn test_extract_url_path_simple() {
    let url = "http://example.com/api";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}

#[test]
fn test_extract_url_path_nested() {
    let url = "http://example.com/api/v1/users";
    let path = extract_url_path(url);
    assert_eq!(path, "/api/v1/users");
}

#[test]
fn test_extract_url_path_with_query() {
    let url = "http://example.com/api?key=value";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}

#[test]
fn test_extract_url_path_with_fragment() {
    let url = "http://example.com/page#section";
    let path = extract_url_path(url);
    assert_eq!(path, "/page");
}

#[test]
fn test_extract_url_path_with_query_and_fragment() {
    let url = "http://example.com/api?key=value#top";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}

#[test]
fn test_extract_url_path_with_port() {
    let url = "http://example.com:8080/api";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}

#[test]
fn test_extract_url_path_https() {
    let url = "https://example.com/secure/api";
    let path = extract_url_path(url);
    assert_eq!(path, "/secure/api");
}

#[test]
fn test_extract_url_path_with_trailing_slash() {
    let url = "http://example.com/api/";
    let path = extract_url_path(url);
    assert_eq!(path, "/api/");
}

#[test]
fn test_extract_url_path_encoded_characters() {
    let url = "http://example.com/api%20test";
    let path = extract_url_path(url);
    assert_eq!(path, "/api%20test");
}

#[test]
fn test_extract_url_path_invalid_url() {
    let url = "not a valid url";
    let path = extract_url_path(url);
    // Should return original string for invalid URLs
    assert_eq!(path, url);
}

#[test]
fn test_extract_url_path_subdomain() {
    let url = "http://api.example.com/v1/users";
    let path = extract_url_path(url);
    assert_eq!(path, "/v1/users");
}

#[test]
fn test_extract_url_path_with_username() {
    let url = "http://user@example.com/api";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}

// ============================================================================
// FollowMode Tests
// ============================================================================

#[test]
fn test_follow_mode_disabled() {
    let mode = FollowMode::Disabled;
    // Just verify we can construct it
    assert!(matches!(mode, FollowMode::Disabled));
}

#[test]
fn test_follow_mode_prompt() {
    let mode = FollowMode::Prompt;
    assert!(matches!(mode, FollowMode::Prompt));
}

#[test]
fn test_follow_mode_auto() {
    let mode = FollowMode::Auto;
    assert!(matches!(mode, FollowMode::Auto));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_extract_url_path_file_extension() {
    let url = "http://example.com/image.jpg";
    let path = extract_url_path(url);
    assert_eq!(path, "/image.jpg");
}

#[test]
fn test_extract_url_path_multiple_slashes() {
    let url = "http://example.com//api//test";
    let path = extract_url_path(url);
    assert_eq!(path, "//api//test");
}

#[test]
fn test_extract_url_path_dot_segments() {
    let url = "http://example.com/./api/../test";
    let path = extract_url_path(url);
    // URL parser normalizes these
    assert!(path.contains("test"));
}

#[test]
fn test_extract_url_path_very_long() {
    let long_path = "/".to_string() + &"a/".repeat(100);
    let url = format!("http://example.com{}", long_path);
    let path = extract_url_path(&url);
    assert!(path.len() > 100);
    assert!(path.starts_with('/'));
}

#[test]
fn test_extract_url_path_unicode() {
    let url = "http://example.com/api/用户";
    let path = extract_url_path(url);
    assert!(path.contains("api"));
}

#[test]
fn test_extract_url_path_special_chars() {
    let url = "http://example.com/api/test-endpoint_v1";
    let path = extract_url_path(url);
    assert_eq!(path, "/api/test-endpoint_v1");
}

#[test]
fn test_extract_url_path_numbers() {
    let url = "http://example.com/api/v1/user/123";
    let path = extract_url_path(url);
    assert_eq!(path, "/api/v1/user/123");
}

#[test]
fn test_extract_url_path_localhost() {
    let url = "http://localhost:3000/api/test";
    let path = extract_url_path(url);
    assert_eq!(path, "/api/test");
}

#[test]
fn test_extract_url_path_ip_address() {
    let url = "http://192.168.1.1/admin";
    let path = extract_url_path(url);
    assert_eq!(path, "/admin");
}

#[test]
fn test_extract_url_path_ipv6() {
    let url = "http://[::1]/api";
    let path = extract_url_path(url);
    assert_eq!(path, "/api");
}
