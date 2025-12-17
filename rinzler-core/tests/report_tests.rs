// Tests for report generation functionality

use rinzler_core::report::{
    FindingData, ReportData, ReportFormat, ScanInfo, SeverityCounts, SitemapNode,
};

// ============================================================================
// Report Format Tests
// ============================================================================

#[test]
fn test_report_format_from_str_text() {
    let format = ReportFormat::from_str("text");
    assert!(matches!(format, Some(ReportFormat::Text)));
}

#[test]
fn test_report_format_from_str_json() {
    let format = ReportFormat::from_str("json");
    assert!(matches!(format, Some(ReportFormat::Json)));
}

#[test]
fn test_report_format_from_str_csv() {
    let format = ReportFormat::from_str("csv");
    assert!(matches!(format, Some(ReportFormat::Csv)));
}

#[test]
fn test_report_format_from_str_html() {
    let format = ReportFormat::from_str("html");
    assert!(matches!(format, Some(ReportFormat::Html)));
}

#[test]
fn test_report_format_from_str_markdown() {
    let format = ReportFormat::from_str("markdown");
    assert!(matches!(format, Some(ReportFormat::Markdown)));
}

#[test]
fn test_report_format_from_str_md() {
    let format = ReportFormat::from_str("md");
    assert!(matches!(format, Some(ReportFormat::Markdown)));
}

#[test]
fn test_report_format_from_str_case_insensitive() {
    assert!(matches!(
        ReportFormat::from_str("TEXT"),
        Some(ReportFormat::Text)
    ));
    assert!(matches!(
        ReportFormat::from_str("Json"),
        Some(ReportFormat::Json)
    ));
    assert!(matches!(
        ReportFormat::from_str("CSV"),
        Some(ReportFormat::Csv)
    ));
}

#[test]
fn test_report_format_from_str_invalid() {
    let format = ReportFormat::from_str("invalid");
    assert!(format.is_none());

    let format = ReportFormat::from_str("pdf");
    assert!(format.is_none());
}

// ============================================================================
// Report Data Structure Tests
// ============================================================================

#[test]
fn test_severity_counts_construction() {
    let counts = SeverityCounts {
        critical: 1,
        high: 2,
        medium: 3,
        low: 4,
        info: 5,
    };

    assert_eq!(counts.critical, 1);
    assert_eq!(counts.high, 2);
    assert_eq!(counts.medium, 3);
    assert_eq!(counts.low, 4);
    assert_eq!(counts.info, 5);
}

#[test]
fn test_severity_counts_zero() {
    let counts = SeverityCounts {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        info: 0,
    };

    assert_eq!(counts.critical, 0);
    assert_eq!(counts.high, 0);
}

#[test]
fn test_finding_data_construction() {
    let finding = FindingData {
        id: 1,
        severity: "high".to_string(),
        title: "SQL Injection".to_string(),
        description: "Possible SQL injection point".to_string(),
        url: "http://example.com/api".to_string(),
        finding_type: "vulnerability".to_string(),
        cwe_id: Some("CWE-89".to_string()),
        owasp_category: Some("A03:2021".to_string()),
        impact: Some("Database compromise".to_string()),
        remediation: Some("Use parameterized queries".to_string()),
    };

    assert_eq!(finding.id, 1);
    assert_eq!(finding.severity, "high");
    assert!(finding.cwe_id.is_some());
    assert!(finding.impact.is_some());
}

#[test]
fn test_finding_data_minimal() {
    let finding = FindingData {
        id: 1,
        severity: "info".to_string(),
        title: "API Endpoint".to_string(),
        description: "Found API endpoint".to_string(),
        url: "http://example.com/api".to_string(),
        finding_type: "interesting_file".to_string(),
        cwe_id: None,
        owasp_category: None,
        impact: None,
        remediation: None,
    };

    assert_eq!(finding.id, 1);
    assert!(finding.cwe_id.is_none());
    assert!(finding.impact.is_none());
}

#[test]
fn test_scan_info_construction() {
    let scan_info = ScanInfo {
        start_time: 1640000000,
        end_time: Some(1640001000),
        status: "completed".to_string(),
        seed_urls: "[\"http://example.com\"]".to_string(),
    };

    assert_eq!(scan_info.start_time, 1640000000);
    assert_eq!(scan_info.end_time, Some(1640001000));
    assert_eq!(scan_info.status, "completed");
}

#[test]
fn test_scan_info_running() {
    let scan_info = ScanInfo {
        start_time: 1640000000,
        end_time: None,
        status: "running".to_string(),
        seed_urls: "[\"http://example.com\"]".to_string(),
    };

    assert!(scan_info.end_time.is_none());
    assert_eq!(scan_info.status, "running");
}

#[test]
fn test_sitemap_node_construction() {
    let node = SitemapNode {
        url: "http://example.com/api".to_string(),
        status_code: 200,
        content_type: Some("application/json".to_string()),
    };

    assert_eq!(node.url, "http://example.com/api");
    assert_eq!(node.status_code, 200);
    assert!(node.content_type.is_some());
}

#[test]
fn test_sitemap_node_minimal() {
    let node = SitemapNode {
        url: "http://example.com/page".to_string(),
        status_code: 404,
        content_type: None,
    };

    assert_eq!(node.status_code, 404);
    assert!(node.content_type.is_none());
}

#[test]
fn test_report_data_construction() {
    let report = ReportData {
        session_id: "test-session".to_string(),
        total_nodes: 10,
        findings: vec![],
        severity_counts: SeverityCounts {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        },
        scan_info: ScanInfo {
            start_time: 1640000000,
            end_time: Some(1640001000),
            status: "completed".to_string(),
            seed_urls: "[\"http://example.com\"]".to_string(),
        },
        sitemap_nodes: None,
    };

    assert_eq!(report.session_id, "test-session");
    assert_eq!(report.total_nodes, 10);
    assert!(report.findings.is_empty());
    assert!(report.sitemap_nodes.is_none());
}

#[test]
fn test_report_data_with_findings() {
    let finding = FindingData {
        id: 1,
        severity: "high".to_string(),
        title: "Test Finding".to_string(),
        description: "Test".to_string(),
        url: "http://example.com".to_string(),
        finding_type: "vulnerability".to_string(),
        cwe_id: None,
        owasp_category: None,
        impact: None,
        remediation: None,
    };

    let report = ReportData {
        session_id: "test-session".to_string(),
        total_nodes: 5,
        findings: vec![finding.clone()],
        severity_counts: SeverityCounts {
            critical: 0,
            high: 1,
            medium: 0,
            low: 0,
            info: 0,
        },
        scan_info: ScanInfo {
            start_time: 1640000000,
            end_time: Some(1640001000),
            status: "completed".to_string(),
            seed_urls: "[\"http://example.com\"]".to_string(),
        },
        sitemap_nodes: None,
    };

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, 1);
    assert_eq!(report.severity_counts.high, 1);
}

#[test]
fn test_report_data_with_sitemap() {
    let sitemap = vec![
        SitemapNode {
            url: "http://example.com/".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
        },
        SitemapNode {
            url: "http://example.com/api".to_string(),
            status_code: 200,
            content_type: Some("application/json".to_string()),
        },
    ];

    let report = ReportData {
        session_id: "test-session".to_string(),
        total_nodes: 2,
        findings: vec![],
        severity_counts: SeverityCounts {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        },
        scan_info: ScanInfo {
            start_time: 1640000000,
            end_time: Some(1640001000),
            status: "completed".to_string(),
            seed_urls: "[\"http://example.com\"]".to_string(),
        },
        sitemap_nodes: Some(sitemap),
    };

    assert!(report.sitemap_nodes.is_some());
    assert_eq!(report.sitemap_nodes.as_ref().unwrap().len(), 2);
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_report_data_json_serialization() {
    let report = ReportData {
        session_id: "test".to_string(),
        total_nodes: 1,
        findings: vec![],
        severity_counts: SeverityCounts {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        },
        scan_info: ScanInfo {
            start_time: 1640000000,
            end_time: Some(1640001000),
            status: "completed".to_string(),
            seed_urls: "[\"http://example.com\"]".to_string(),
        },
        sitemap_nodes: None,
    };

    let json = serde_json::to_string(&report);
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("test"));
    assert!(json_str.contains("completed"));
}

#[test]
fn test_finding_data_json_serialization() {
    let finding = FindingData {
        id: 1,
        severity: "high".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        url: "http://example.com".to_string(),
        finding_type: "vulnerability".to_string(),
        cwe_id: Some("CWE-89".to_string()),
        owasp_category: Some("A03:2021".to_string()),
        impact: Some("High impact".to_string()),
        remediation: Some("Fix it".to_string()),
    };

    let json = serde_json::to_string(&finding);
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("CWE-89"));
    assert!(json_str.contains("A03:2021"));
}

#[test]
fn test_finding_data_json_optional_fields() {
    let finding = FindingData {
        id: 1,
        severity: "info".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        url: "http://example.com".to_string(),
        finding_type: "interesting_file".to_string(),
        cwe_id: None,
        owasp_category: None,
        impact: None,
        remediation: None,
    };

    let json = serde_json::to_string(&finding).unwrap();

    // Optional None fields should not be in JSON
    assert!(!json.contains("cwe_id"));
    assert!(!json.contains("impact"));
}

#[test]
fn test_severity_counts_json_serialization() {
    let counts = SeverityCounts {
        critical: 1,
        high: 2,
        medium: 3,
        low: 4,
        info: 5,
    };

    let json = serde_json::to_string(&counts);
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("\"critical\":1"));
    assert!(json_str.contains("\"high\":2"));
}

// ============================================================================
// Clone Tests
// ============================================================================

#[test]
fn test_report_data_clone() {
    let report = ReportData {
        session_id: "test".to_string(),
        total_nodes: 5,
        findings: vec![],
        severity_counts: SeverityCounts {
            critical: 1,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        },
        scan_info: ScanInfo {
            start_time: 1640000000,
            end_time: None,
            status: "running".to_string(),
            seed_urls: "[]".to_string(),
        },
        sitemap_nodes: None,
    };

    let cloned = report.clone();
    assert_eq!(cloned.session_id, report.session_id);
    assert_eq!(cloned.total_nodes, report.total_nodes);
    assert_eq!(
        cloned.severity_counts.critical,
        report.severity_counts.critical
    );
}

#[test]
fn test_finding_data_clone() {
    let finding = FindingData {
        id: 1,
        severity: "high".to_string(),
        title: "Test".to_string(),
        description: "Desc".to_string(),
        url: "http://example.com".to_string(),
        finding_type: "vuln".to_string(),
        cwe_id: Some("CWE-89".to_string()),
        owasp_category: None,
        impact: None,
        remediation: None,
    };

    let cloned = finding.clone();
    assert_eq!(cloned.id, finding.id);
    assert_eq!(cloned.severity, finding.severity);
    assert_eq!(cloned.cwe_id, finding.cwe_id);
}
