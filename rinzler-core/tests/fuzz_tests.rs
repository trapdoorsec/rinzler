// Tests for fuzzing functionality

use rinzler_core::fuzz::{FuzzSource, build_test_url, extract_base_url, load_wordlist};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_build_test_url_basic() {
    let base = "http://example.com";
    let word = "api";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "http://example.com/api");
}

#[test]
fn test_build_test_url_with_trailing_slash() {
    let base = "http://example.com/";
    let word = "test";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "http://example.com/test");
}

#[test]
fn test_build_test_url_with_path() {
    let base = "http://example.com/api";
    let word = "users";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "http://example.com/api/users");
}

#[test]
fn test_build_test_url_with_leading_slash_word() {
    let base = "http://example.com";
    let word = "/admin";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "http://example.com/admin");
}

#[test]
fn test_build_test_url_with_port() {
    let base = "http://example.com:8080/api";
    let word = "v1";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "http://example.com:8080/api/v1");
}

#[test]
fn test_build_test_url_https() {
    let base = "https://api.example.com";
    let word = "endpoint";
    let result = build_test_url(base, word).unwrap();
    assert_eq!(result, "https://api.example.com/endpoint");
}

#[test]
fn test_build_test_url_invalid_base() {
    let base = "not a url";
    let word = "test";
    let result = build_test_url(base, word);
    assert!(result.is_err());
}

#[test]
fn test_extract_base_url_basic() {
    let url = "http://example.com/api/users";
    let result = extract_base_url(url).unwrap();
    assert_eq!(result, "http://example.com/api/users");
}

#[test]
fn test_extract_base_url_with_query() {
    let url = "http://example.com/api?test=1";
    let result = extract_base_url(url).unwrap();
    assert_eq!(result, "http://example.com/api");
}

#[test]
fn test_extract_base_url_with_fragment() {
    let url = "http://example.com/page#section";
    let result = extract_base_url(url).unwrap();
    assert_eq!(result, "http://example.com/page");
}

#[test]
fn test_extract_base_url_with_query_and_fragment() {
    let url = "http://example.com/api?key=val#top";
    let result = extract_base_url(url).unwrap();
    assert_eq!(result, "http://example.com/api");
}

#[test]
fn test_load_wordlist_basic() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(&wordlist_path, "api\ntest\nadmin\nconfig").unwrap();

    let words = load_wordlist(&wordlist_path).unwrap();
    assert_eq!(words.len(), 4);
    assert_eq!(words[0], "api");
    assert_eq!(words[1], "test");
    assert_eq!(words[2], "admin");
    assert_eq!(words[3], "config");
}

#[test]
fn test_load_wordlist_with_comments() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(
        &wordlist_path,
        "# Comment line\napi\n# Another comment\ntest\nadmin",
    )
    .unwrap();

    let words = load_wordlist(&wordlist_path).unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0], "api");
    assert_eq!(words[1], "test");
    assert_eq!(words[2], "admin");
}

#[test]
fn test_load_wordlist_with_empty_lines() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(&wordlist_path, "api\n\ntest\n   \nadmin\n\n").unwrap();

    let words = load_wordlist(&wordlist_path).unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0], "api");
    assert_eq!(words[1], "test");
    assert_eq!(words[2], "admin");
}

#[test]
fn test_load_wordlist_with_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(&wordlist_path, "  api  \n\ttest\t\n  admin  ").unwrap();

    let words = load_wordlist(&wordlist_path).unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0], "api");
    assert_eq!(words[1], "test");
    assert_eq!(words[2], "admin");
}

#[test]
fn test_load_wordlist_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(&wordlist_path, "").unwrap();

    let result = load_wordlist(&wordlist_path);
    assert!(result.is_err());
}

#[test]
fn test_load_wordlist_only_comments() {
    let temp_dir = TempDir::new().unwrap();
    let wordlist_path = temp_dir.path().join("test_wordlist.txt");

    fs::write(&wordlist_path, "# Comment 1\n# Comment 2\n# Comment 3").unwrap();

    let result = load_wordlist(&wordlist_path);
    assert!(result.is_err());
}

#[test]
fn test_load_wordlist_nonexistent_file() {
    let wordlist_path = PathBuf::from("/nonexistent/path/wordlist.txt");
    let result = load_wordlist(&wordlist_path);
    assert!(result.is_err());
}

#[test]
fn test_fuzz_source_clone() {
    let source = FuzzSource::Initial;
    let cloned = source.clone();
    assert!(matches!(cloned, FuzzSource::Initial));

    let source = FuzzSource::Database;
    let cloned = source.clone();
    assert!(matches!(cloned, FuzzSource::Database));

    let source = FuzzSource::Discovered;
    let cloned = source.clone();
    assert!(matches!(cloned, FuzzSource::Discovered));
}

#[test]
fn test_fuzz_source_equality() {
    assert_eq!(FuzzSource::Initial, FuzzSource::Initial);
    assert_eq!(FuzzSource::Database, FuzzSource::Database);
    assert_eq!(FuzzSource::Discovered, FuzzSource::Discovered);

    assert_ne!(FuzzSource::Initial, FuzzSource::Database);
    assert_ne!(FuzzSource::Database, FuzzSource::Discovered);
    assert_ne!(FuzzSource::Initial, FuzzSource::Discovered);
}
