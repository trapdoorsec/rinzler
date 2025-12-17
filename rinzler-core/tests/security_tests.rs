// Tests for security analysis functionality

use rinzler_core::data::{FindingType, Severity};
use rinzler_core::security::{
    analyze_crawl_result, check_error_messages, check_insecure_transport, check_interesting_files,
};
use rinzler_scanner::result::CrawlResult;

fn create_test_result(url: &str, status_code: u16, content_type: Option<&str>) -> CrawlResult {
    let mut result = CrawlResult::new(url.to_string());
    result.status_code = status_code;
    result.content_type = content_type.map(String::from);
    result
}

// ============================================================================
// Insecure Transport Tests
// ============================================================================

#[test]
fn test_check_insecure_transport_http() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Medium);
    assert!(matches!(
        findings[0].finding_type,
        FindingType::InsecureTransport
    ));
    assert!(findings[0].title.contains("Insecure Transport"));
}

#[test]
fn test_check_insecure_transport_https() {
    let result = create_test_result("https://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_insecure_transport_localhost() {
    let result = create_test_result("http://localhost/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_insecure_transport_127() {
    let result = create_test_result("http://127.0.0.1/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_insecure_transport_with_port() {
    let result = create_test_result("http://example.com:8080/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
}

// ============================================================================
// Interesting Files Tests
// ============================================================================

#[test]
fn test_check_interesting_files_git() {
    let result = create_test_result("http://example.com/.git/config", 200, Some("text/plain"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(matches!(
        findings[0].finding_type,
        FindingType::InterestingFile
    ));
    assert!(findings[0].title.contains("Git"));
}

#[test]
fn test_check_interesting_files_env() {
    let result = create_test_result("http://example.com/.env", 200, Some("text/plain"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Critical);
    assert!(findings[0].title.contains("Environment"));
}

#[test]
fn test_check_interesting_files_aws() {
    let result = create_test_result(
        "http://example.com/.aws/credentials",
        200,
        Some("text/plain"),
    );
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Critical);
    assert!(findings[0].title.contains("AWS"));
}

#[test]
fn test_check_interesting_files_sql() {
    let result = create_test_result("http://example.com/dump.sql", 200, Some("text/plain"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(findings[0].title.contains("SQL"));
}

#[test]
fn test_check_interesting_files_bak() {
    let result = create_test_result("http://example.com/index.php.bak", 200, Some("text/plain"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Medium);
    assert!(findings[0].title.contains("Backup"));
}

#[test]
fn test_check_interesting_files_config() {
    let result = create_test_result("http://example.com/web.config", 200, Some("text/xml"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(findings[0].title.contains("Configuration"));
}

#[test]
fn test_check_interesting_files_phpinfo() {
    let result = create_test_result("http://example.com/phpinfo.php", 200, Some("text/html"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Info);
    assert!(findings[0].title.contains("PHP Info"));
}

#[test]
fn test_check_interesting_files_admin() {
    let result = create_test_result("http://example.com/admin", 200, Some("text/html"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Info);
    assert!(findings[0].title.contains("Admin"));
}

#[test]
fn test_check_interesting_files_api() {
    let result = create_test_result(
        "http://example.com/api/users",
        200,
        Some("application/json"),
    );
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Info);
    assert!(findings[0].title.contains("API"));
}

#[test]
fn test_check_interesting_files_404() {
    let result = create_test_result("http://example.com/.env", 404, Some("text/html"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_interesting_files_500() {
    let result = create_test_result("http://example.com/.git/config", 500, Some("text/html"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_interesting_files_case_insensitive() {
    let result = create_test_result("http://example.com/.GIT/CONFIG", 200, Some("text/plain"));
    let findings = check_interesting_files(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("Git"));
}

// ============================================================================
// Error Message Tests
// ============================================================================

#[test]
fn test_check_error_messages_500() {
    let result = create_test_result("http://example.com/api", 500, Some("text/html"));
    let findings = check_error_messages(&result, 1);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
    assert!(matches!(
        findings[0].finding_type,
        FindingType::InformationDisclosure
    ));
    assert!(findings[0].title.contains("500"));
}

#[test]
fn test_check_error_messages_502() {
    let result = create_test_result("http://example.com/api", 502, Some("text/html"));
    let findings = check_error_messages(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("502"));
}

#[test]
fn test_check_error_messages_503() {
    let result = create_test_result("http://example.com/api", 503, Some("text/html"));
    let findings = check_error_messages(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("503"));
}

#[test]
fn test_check_error_messages_200() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_error_messages(&result, 1);

    assert_eq!(findings.len(), 0);
}

#[test]
fn test_check_error_messages_404() {
    let result = create_test_result("http://example.com/api", 404, Some("text/html"));
    let findings = check_error_messages(&result, 1);

    assert_eq!(findings.len(), 0);
}

// ============================================================================
// Integrated Analysis Tests
// ============================================================================

#[test]
fn test_analyze_crawl_result_multiple_findings() {
    let result = create_test_result("http://example.com/.env", 200, Some("text/plain"));
    let findings = analyze_crawl_result(&result, 1);

    // Should find both insecure transport and interesting file
    assert!(findings.len() >= 2);

    let has_insecure = findings
        .iter()
        .any(|f| matches!(f.finding_type, FindingType::InsecureTransport));
    let has_interesting = findings
        .iter()
        .any(|f| matches!(f.finding_type, FindingType::InterestingFile));

    assert!(has_insecure);
    assert!(has_interesting);
}

#[test]
fn test_analyze_crawl_result_https_safe() {
    let result = create_test_result(
        "https://example.com/api/users",
        200,
        Some("application/json"),
    );
    let findings = analyze_crawl_result(&result, 1);

    // Should only find API endpoint (info level)
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Info);
}

#[test]
fn test_analyze_crawl_result_server_error() {
    let result = create_test_result("http://example.com/api", 500, Some("text/html"));
    let findings = analyze_crawl_result(&result, 1);

    // Should find both insecure transport and server error
    assert!(findings.len() >= 2);

    let has_error = findings
        .iter()
        .any(|f| matches!(f.finding_type, FindingType::InformationDisclosure));

    assert!(has_error);
}

#[test]
fn test_analyze_crawl_result_clean() {
    let result = create_test_result("https://example.com/about", 200, Some("text/html"));
    let findings = analyze_crawl_result(&result, 1);

    // Should have no findings (clean endpoint)
    assert_eq!(findings.len(), 0);
}

// ============================================================================
// Finding Field Tests
// ============================================================================

#[test]
fn test_finding_has_cwe() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].cwe_id.is_some());
    assert!(findings[0].cwe_id.as_ref().unwrap().contains("CWE"));
}

#[test]
fn test_finding_has_owasp() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].owasp_category.is_some());
    assert!(
        findings[0]
            .owasp_category
            .as_ref()
            .unwrap()
            .contains("2021")
    );
}

#[test]
fn test_finding_has_remediation() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].remediation.is_some());
    assert!(!findings[0].remediation.as_ref().unwrap().is_empty());
}

#[test]
fn test_finding_has_impact() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].impact.is_some());
    assert!(!findings[0].impact.as_ref().unwrap().is_empty());
}

#[test]
fn test_finding_has_evidence() {
    let result = create_test_result("http://example.com/api", 200, Some("text/html"));
    let findings = check_insecure_transport(&result, 1);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].evidence.is_some());
    assert!(findings[0].evidence.as_ref().unwrap().contains("http"));
}
