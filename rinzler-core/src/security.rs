// Passive security checks for crawled endpoints

use crate::data::{Finding, FindingType, Severity};
use rinzler_scanner::result::CrawlResult;
use url::Url;

pub fn check_security_headers(result: &CrawlResult, node_id: i64) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Check for missing security headers (only for successful HTML responses)
    if result.status_code >= 200
        && result.status_code < 300
        && let Some(ref content_type) = result.content_type
        && content_type.contains("text/html")
    {
        // Check for missing security headers
        // Note: In real implementation, we'd need access to response headers
        // For now, this is a placeholder structure

        // X-Frame-Options missing
        findings.push(Finding {
                    node_id,
                    finding_type: FindingType::SecurityHeaderMissing,
                    severity: Severity::Low,
                    title: "Missing X-Frame-Options Header".to_string(),
                    description: "The X-Frame-Options header is not set, which may allow clickjacking attacks.".to_string(),
                    impact: Some("Attackers could embed this page in an iframe on a malicious site to perform clickjacking attacks.".to_string()),
                    remediation: Some("Add 'X-Frame-Options: DENY' or 'X-Frame-Options: SAMEORIGIN' header to HTTP responses.".to_string()),
                    evidence: None,
                    cwe_id: Some("CWE-1021".to_string()),
                    owasp_category: Some("A05:2021 - Security Misconfiguration".to_string()),
                });
    }

    findings
}

pub fn check_insecure_transport(result: &CrawlResult, node_id: i64) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(parsed_url) = Url::parse(&result.url)
        && parsed_url.scheme() == "http"
    {
        // Check if this is not localhost
        if let Some(host) = parsed_url.host_str()
            && !host.starts_with("127.")
            && host != "localhost"
        {
            findings.push(Finding {
                        node_id,
                        finding_type: FindingType::InsecureTransport,
                        severity: Severity::Medium,
                        title: "Insecure Transport (HTTP)".to_string(),
                        description: format!("The endpoint {} is served over HTTP instead of HTTPS.", result.url),
                        impact: Some("Data transmitted over HTTP can be intercepted and read by attackers. Sensitive information like credentials, session tokens, and personal data may be exposed.".to_string()),
                        remediation: Some("Enable HTTPS for this endpoint and redirect all HTTP traffic to HTTPS.".to_string()),
                        evidence: Some(format!("{{\"url\": \"{}\", \"scheme\": \"http\"}}", result.url)),
                        cwe_id: Some("CWE-319".to_string()),
                        owasp_category: Some("A02:2021 - Cryptographic Failures".to_string()),
                    });
        }
    }

    findings
}

pub fn check_interesting_files(result: &CrawlResult, node_id: i64) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(parsed_url) = Url::parse(&result.url) {
        let path = parsed_url.path().to_lowercase();

        // Check for common interesting files
        let interesting_patterns = vec![
            (".git/", "Git Repository Exposed", Severity::High, "CWE-538"),
            (
                ".env",
                "Environment File Exposed",
                Severity::Critical,
                "CWE-200",
            ),
            (
                ".git/config",
                "Git Configuration Exposed",
                Severity::High,
                "CWE-538",
            ),
            (
                "/.aws/",
                "AWS Credentials Directory",
                Severity::Critical,
                "CWE-200",
            ),
            (
                "/backup",
                "Backup File Accessible",
                Severity::Medium,
                "CWE-530",
            ),
            (".sql", "SQL Dump File", Severity::High, "CWE-530"),
            (".bak", "Backup File", Severity::Medium, "CWE-530"),
            (
                "web.config",
                "Configuration File Exposed",
                Severity::High,
                "CWE-215",
            ),
            ("phpinfo.php", "PHP Info Page", Severity::Info, "CWE-200"),
            ("/admin", "Admin Interface", Severity::Info, "CWE-200"),
            ("/api/", "API Endpoint", Severity::Info, "CWE-200"),
        ];

        for (pattern, title, severity, cwe) in interesting_patterns {
            if path.contains(pattern) && result.status_code >= 200 && result.status_code < 300 {
                findings.push(Finding {
                    node_id,
                    finding_type: FindingType::InterestingFile,
                    severity,
                    title: title.to_string(),
                    description: format!("Discovered potentially sensitive file or directory: {}", result.url),
                    impact: Some("This file or directory may contain sensitive information or provide attack surface.".to_string()),
                    remediation: Some("Review if this resource should be publicly accessible. Consider removing or restricting access.".to_string()),
                    evidence: Some(format!("{{\"url\": \"{}\", \"status_code\": {}}}", result.url, result.status_code)),
                    cwe_id: Some(cwe.to_string()),
                    owasp_category: Some("A01:2021 - Broken Access Control".to_string()),
                });
                break; // Only report once per URL
            }
        }
    }

    findings
}

pub fn check_error_messages(result: &CrawlResult, node_id: i64) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Check for error status codes that might reveal information
    if result.status_code >= 500 && result.status_code < 600 {
        findings.push(Finding {
            node_id,
            finding_type: FindingType::InformationDisclosure,
            severity: Severity::Low,
            title: format!("Server Error - {}", result.status_code),
            description: format!("Server returned error code {} for {}. Error pages may leak sensitive information.", result.status_code, result.url),
            impact: Some("Server errors may expose stack traces, file paths, or other sensitive system information.".to_string()),
            remediation: Some("Configure custom error pages that don't reveal system details.".to_string()),
            evidence: Some(format!("{{\"url\": \"{}\", \"status_code\": {}}}", result.url, result.status_code)),
            cwe_id: Some("CWE-209".to_string()),
            owasp_category: Some("A05:2021 - Security Misconfiguration".to_string()),
        });
    }

    findings
}

pub fn analyze_crawl_result(result: &CrawlResult, node_id: i64) -> Vec<Finding> {
    let mut all_findings = Vec::new();

    // Run all passive checks
    all_findings.extend(check_insecure_transport(result, node_id));
    all_findings.extend(check_interesting_files(result, node_id));
    all_findings.extend(check_error_messages(result, node_id));
    // check_security_headers would need actual headers from the scanner
    // all_findings.extend(check_security_headers(result, node_id));

    all_findings
}
