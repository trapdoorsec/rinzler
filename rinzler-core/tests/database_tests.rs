// Tests for database functionality

use rinzler_core::data::{CrawlNode, Database, Finding, FindingType, ServiceType, Severity};
use tempfile::TempDir;

fn create_test_db() -> (TempDir, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    (temp_dir, db)
}

// ============================================================================
// Database Creation Tests
// ============================================================================

#[test]
fn test_database_creation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path);
    assert!(db.is_ok());
    assert!(db_path.exists());
}

#[test]
fn test_database_exists() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    assert!(!Database::exists(&db_path));

    let _db = Database::new(&db_path).unwrap();
    assert!(Database::exists(&db_path));
}

#[test]
fn test_database_drop() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let _db = Database::new(&db_path).unwrap();
    assert!(Database::exists(&db_path));

    Database::drop(&db_path);
    assert!(!Database::exists(&db_path));
}

// ============================================================================
// Session Tests
// ============================================================================

#[test]
fn test_create_session() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    assert!(!session_id.is_empty());
}

#[test]
fn test_create_multiple_sessions() {
    let (_temp_dir, db) = create_test_db();

    let session1 = db
        .create_session("crawl", "[\"http://example1.com\"]")
        .unwrap();
    let session2 = db
        .create_session("fuzz", "[\"http://example2.com\"]")
        .unwrap();

    assert_ne!(session1, session2);
}

#[test]
fn test_complete_session() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let result = db.complete_session(&session_id);

    assert!(result.is_ok());
}

// ============================================================================
// Node Tests
// ============================================================================

#[test]
fn test_insert_node() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    let node = CrawlNode {
        url: "http://example.com/api".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: Some("application/json".to_string()),
        content_length: Some(1024),
        response_time_ms: Some(150),
        title: Some("API Endpoint".to_string()),
        forms_count: 0,
        service_type: Some(ServiceType::RestApi),
        headers: Some("{}".to_string()),
        body_sample: Some("{}".to_string()),
    };

    let node_id = db.insert_node(&map_id, &node).unwrap();
    assert!(node_id > 0);
}

#[test]
fn test_insert_multiple_nodes() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    let node1 = CrawlNode {
        url: "http://example.com/api".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: Some("application/json".to_string()),
        content_length: Some(1024),
        response_time_ms: Some(150),
        title: None,
        forms_count: 0,
        service_type: Some(ServiceType::RestApi),
        headers: None,
        body_sample: None,
    };

    let node2 = CrawlNode {
        url: "http://example.com/login".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: Some("text/html".to_string()),
        content_length: Some(2048),
        response_time_ms: Some(200),
        title: Some("Login".to_string()),
        forms_count: 1,
        service_type: Some(ServiceType::Web),
        headers: None,
        body_sample: None,
    };

    let node_id1 = db.insert_node(&map_id, &node1).unwrap();
    let node_id2 = db.insert_node(&map_id, &node2).unwrap();

    assert!(node_id1 > 0);
    assert!(node_id2 > 0);
    assert_ne!(node_id1, node_id2);
}

// ============================================================================
// Finding Tests
// ============================================================================

#[test]
fn test_insert_finding() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    let node = CrawlNode {
        url: "http://example.com/api".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: Some("application/json".to_string()),
        content_length: Some(1024),
        response_time_ms: Some(150),
        title: None,
        forms_count: 0,
        service_type: Some(ServiceType::RestApi),
        headers: None,
        body_sample: None,
    };

    let node_id = db.insert_node(&map_id, &node).unwrap();

    let finding = Finding {
        node_id,
        finding_type: FindingType::InsecureTransport,
        severity: Severity::Medium,
        title: "Insecure Transport".to_string(),
        description: "HTTP instead of HTTPS".to_string(),
        impact: Some("Data can be intercepted".to_string()),
        remediation: Some("Use HTTPS".to_string()),
        evidence: Some("{\"scheme\": \"http\"}".to_string()),
        cwe_id: Some("CWE-319".to_string()),
        owasp_category: Some("A02:2021".to_string()),
    };

    let result = db.insert_finding(&session_id, &finding);
    assert!(result.is_ok());
}

#[test]
fn test_insert_multiple_findings() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    let node = CrawlNode {
        url: "http://example.com/.env".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: Some("text/plain".to_string()),
        content_length: Some(512),
        response_time_ms: Some(100),
        title: None,
        forms_count: 0,
        service_type: None,
        headers: None,
        body_sample: None,
    };

    let node_id = db.insert_node(&map_id, &node).unwrap();

    let finding1 = Finding {
        node_id,
        finding_type: FindingType::InsecureTransport,
        severity: Severity::Medium,
        title: "Insecure Transport".to_string(),
        description: "HTTP instead of HTTPS".to_string(),
        impact: None,
        remediation: None,
        evidence: None,
        cwe_id: Some("CWE-319".to_string()),
        owasp_category: None,
    };

    let finding2 = Finding {
        node_id,
        finding_type: FindingType::InterestingFile,
        severity: Severity::Critical,
        title: "Environment File Exposed".to_string(),
        description: "Discovered .env file".to_string(),
        impact: Some("Credentials may be exposed".to_string()),
        remediation: Some("Remove .env from public access".to_string()),
        evidence: None,
        cwe_id: Some("CWE-200".to_string()),
        owasp_category: Some("A01:2021".to_string()),
    };

    let result1 = db.insert_finding(&session_id, &finding1);
    let result2 = db.insert_finding(&session_id, &finding2);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn test_get_findings_count_by_severity() {
    let (_temp_dir, db) = create_test_db();

    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    let node = CrawlNode {
        url: "http://example.com/test".to_string(),
        domain: "example.com".to_string(),
        status_code: 200,
        content_type: None,
        content_length: None,
        response_time_ms: None,
        title: None,
        forms_count: 0,
        service_type: None,
        headers: None,
        body_sample: None,
    };

    let node_id = db.insert_node(&map_id, &node).unwrap();

    // Insert findings with different severities
    let critical_finding = Finding {
        node_id,
        finding_type: FindingType::InterestingFile,
        severity: Severity::Critical,
        title: "Critical Issue".to_string(),
        description: "Critical".to_string(),
        impact: None,
        remediation: None,
        evidence: None,
        cwe_id: None,
        owasp_category: None,
    };

    let medium_finding = Finding {
        node_id,
        finding_type: FindingType::InsecureTransport,
        severity: Severity::Medium,
        title: "Medium Issue".to_string(),
        description: "Medium".to_string(),
        impact: None,
        remediation: None,
        evidence: None,
        cwe_id: None,
        owasp_category: None,
    };

    db.insert_finding(&session_id, &critical_finding).unwrap();
    db.insert_finding(&session_id, &medium_finding).unwrap();

    let severity_counts = db.get_findings_count_by_severity(&session_id).unwrap();

    // Verify we have severity counts
    assert!(!severity_counts.is_empty());

    // Check critical count
    let critical_count = severity_counts
        .iter()
        .find(|(sev, _)| sev == "critical")
        .map(|(_, count)| *count);
    assert_eq!(critical_count, Some(1));

    // Check medium count
    let medium_count = severity_counts
        .iter()
        .find(|(sev, _)| sev == "medium")
        .map(|(_, count)| *count);
    assert_eq!(medium_count, Some(1));
}

// ============================================================================
// Enum Conversion Tests
// ============================================================================

#[test]
fn test_severity_as_str() {
    assert_eq!(Severity::Critical.as_str(), "critical");
    assert_eq!(Severity::High.as_str(), "high");
    assert_eq!(Severity::Medium.as_str(), "medium");
    assert_eq!(Severity::Low.as_str(), "low");
    assert_eq!(Severity::Info.as_str(), "info");
}

#[test]
fn test_finding_type_as_str() {
    assert_eq!(FindingType::Vulnerability.as_str(), "vulnerability");
    assert_eq!(FindingType::Misconfiguration.as_str(), "misconfiguration");
    assert_eq!(
        FindingType::InformationDisclosure.as_str(),
        "information_disclosure"
    );
    assert_eq!(FindingType::InterestingFile.as_str(), "interesting_file");
    assert_eq!(
        FindingType::SecurityHeaderMissing.as_str(),
        "security_header_missing"
    );
    assert_eq!(
        FindingType::InsecureTransport.as_str(),
        "insecure_transport"
    );
    assert_eq!(
        FindingType::AuthenticationIssue.as_str(),
        "authentication_issue"
    );
    assert_eq!(
        FindingType::AuthorizationIssue.as_str(),
        "authorization_issue"
    );
    assert_eq!(FindingType::InjectionPoint.as_str(), "injection_point");
    assert_eq!(FindingType::Other.as_str(), "other");
}

#[test]
fn test_service_type_as_str() {
    assert_eq!(ServiceType::Web.as_str(), "web");
    assert_eq!(ServiceType::RestApi.as_str(), "rest_api");
    assert_eq!(ServiceType::GraphQL.as_str(), "graphql");
    assert_eq!(ServiceType::Soap.as_str(), "soap");
    assert_eq!(ServiceType::WebSocket.as_str(), "websocket");
    assert_eq!(ServiceType::Static.as_str(), "static");
    assert_eq!(ServiceType::Redirect.as_str(), "redirect");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_workflow() {
    let (_temp_dir, db) = create_test_db();

    // Create session
    let session_id = db
        .create_session("crawl", "[\"http://example.com\"]")
        .unwrap();
    let map_id = db.create_map(&session_id).unwrap();

    // Insert multiple nodes
    for i in 1..=5 {
        let node = CrawlNode {
            url: format!("http://example.com/page{}", i),
            domain: "example.com".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            content_length: Some(1024),
            response_time_ms: Some(100 + i as u64),
            title: Some(format!("Page {}", i)),
            forms_count: 0,
            service_type: Some(ServiceType::Web),
            headers: None,
            body_sample: None,
        };

        let node_id = db.insert_node(&map_id, &node).unwrap();

        // Add a finding for each node
        let finding = Finding {
            node_id,
            finding_type: FindingType::InsecureTransport,
            severity: Severity::Medium,
            title: "Insecure Transport".to_string(),
            description: "HTTP used".to_string(),
            impact: None,
            remediation: None,
            evidence: None,
            cwe_id: Some("CWE-319".to_string()),
            owasp_category: None,
        };

        db.insert_finding(&session_id, &finding).unwrap();
    }

    // Complete session
    db.complete_session(&session_id).unwrap();

    // Verify findings count
    let severity_counts = db.get_findings_count_by_severity(&session_id).unwrap();
    let medium_count = severity_counts
        .iter()
        .find(|(sev, _)| sev == "medium")
        .map(|(_, count)| *count);
    assert_eq!(medium_count, Some(5));
}
