use rusqlite::{Connection, OptionalExtension, Result, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FindingType {
    Vulnerability,
    Misconfiguration,
    InformationDisclosure,
    InterestingFile,
    SecurityHeaderMissing,
    InsecureTransport,
    AuthenticationIssue,
    AuthorizationIssue,
    InjectionPoint,
    Other,
}

impl FindingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingType::Vulnerability => "vulnerability",
            FindingType::Misconfiguration => "misconfiguration",
            FindingType::InformationDisclosure => "information_disclosure",
            FindingType::InterestingFile => "interesting_file",
            FindingType::SecurityHeaderMissing => "security_header_missing",
            FindingType::InsecureTransport => "insecure_transport",
            FindingType::AuthenticationIssue => "authentication_issue",
            FindingType::AuthorizationIssue => "authorization_issue",
            FindingType::InjectionPoint => "injection_point",
            FindingType::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    Web,
    RestApi,
    GraphQL,
    Soap,
    WebSocket,
    Static,
    Redirect,
}

impl ServiceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceType::Web => "web",
            ServiceType::RestApi => "rest_api",
            ServiceType::GraphQL => "graphql",
            ServiceType::Soap => "soap",
            ServiceType::WebSocket => "websocket",
            ServiceType::Static => "static",
            ServiceType::Redirect => "redirect",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrawlNode {
    pub url: String,
    pub domain: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub response_time_ms: Option<u64>,
    pub title: Option<String>,
    pub forms_count: usize,
    pub service_type: Option<ServiceType>,
    pub headers: Option<String>, // JSON
    pub body_sample: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub node_id: i64,
    pub finding_type: FindingType,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub evidence: Option<String>, // JSON
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl Database {
    pub fn drop(path: &Path) {
        fs::remove_file(path).unwrap();
    }
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Optimize for concurrent writes
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;  -- 64MB cache
            PRAGMA temp_store = MEMORY;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            -- Scan sessions
            CREATE TABLE IF NOT EXISTS crawl_sessions (
    id TEXT PRIMARY KEY,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    status TEXT NOT NULL CHECK(status IN ('running', 'completed', 'failed', 'cancelled')),
    scan_type TEXT NOT NULL CHECK(scan_type IN ('crawl', 'fuzz', 'manual')),
    seed_urls TEXT NOT NULL,  -- JSON array
    configuration TEXT        -- JSON configuration used
);

CREATE TABLE IF NOT EXISTS maps (
    id TEXT PRIMARY KEY,
    session_id TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES crawl_sessions(id) ON DELETE CASCADE
);

-- Nodes in the graph
CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id TEXT NOT NULL,
    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    node_type TEXT NOT NULL CHECK(node_type IN ('root_host', 'endpoint', 'external_host', 'api_endpoint')),

    -- Crawl metadata
    status TEXT NOT NULL DEFAULT 'pending',
    depth INTEGER NOT NULL DEFAULT 0,
    discovered_at INTEGER NOT NULL,
    last_crawled INTEGER,
    response_code INTEGER,
    response_time_ms INTEGER,
    content_hash TEXT,
    content_type TEXT,
    content_length INTEGER,
    title TEXT,

    -- Service identification
    service_type TEXT CHECK(service_type IN ('web', 'rest_api', 'graphql', 'soap', 'websocket', 'static', 'redirect')),
    http_methods TEXT,        -- JSON array of supported methods
    requires_auth BOOLEAN,

    -- Response details
    headers TEXT,             -- JSON object of response headers
    body_sample TEXT,         -- First 1KB of response for analysis
    technologies TEXT,        -- JSON array of detected technologies

    -- Form/parameter metadata
    forms_count INTEGER DEFAULT 0,
    inputs_count INTEGER DEFAULT 0,
    parameters TEXT,          -- JSON array of discovered parameters

    -- Graph positioning
    position_x REAL,
    position_y REAL,

    FOREIGN KEY(map_id) REFERENCES maps(id) ON DELETE CASCADE,
    UNIQUE(map_id, url)
);

CREATE INDEX IF NOT EXISTS idx_nodes_map ON nodes(map_id);
CREATE INDEX IF NOT EXISTS idx_nodes_domain ON nodes(map_id, domain);
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(map_id, status);
CREATE INDEX IF NOT EXISTS idx_nodes_service_type ON nodes(service_type);
CREATE INDEX IF NOT EXISTS idx_nodes_response_code ON nodes(response_code);

-- Edges in the graph
CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id TEXT NOT NULL,
    source_node_id INTEGER NOT NULL,
    target_node_id INTEGER NOT NULL,

    edge_type TEXT NOT NULL CHECK(edge_type IN (
        'navigation',      -- standard href link
        'reference',       -- cross-domain link
        'redirect',        -- HTTP redirect
        'form_action',     -- form submission target
        'api_call',        -- XHR/fetch endpoint
        'resource'         -- CSS/JS/image
    )),

    -- Edge metadata
    discovered_at INTEGER NOT NULL,
    link_text TEXT,
    context TEXT,
    http_method TEXT,
    weight REAL DEFAULT 1.0,

    FOREIGN KEY(map_id) REFERENCES maps(id) ON DELETE CASCADE,
    FOREIGN KEY(source_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY(target_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
    UNIQUE(source_node_id, target_node_id, edge_type)
);

CREATE INDEX IF NOT EXISTS idx_edges_map ON edges(map_id);
CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_node_id);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_node_id);

-- Findings and vulnerabilities
CREATE TABLE IF NOT EXISTS findings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    node_id INTEGER NOT NULL,

    -- Classification
    finding_type TEXT NOT NULL CHECK(finding_type IN (
        'vulnerability',
        'misconfiguration',
        'information_disclosure',
        'interesting_file',
        'security_header_missing',
        'insecure_transport',
        'authentication_issue',
        'authorization_issue',
        'injection_point',
        'other'
    )),

    severity TEXT NOT NULL CHECK(severity IN ('critical', 'high', 'medium', 'low', 'info')),
    confidence TEXT NOT NULL CHECK(confidence IN ('confirmed', 'likely', 'possible', 'false_positive')),

    -- Details
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    impact TEXT,
    remediation TEXT,

    -- Evidence
    evidence TEXT,            -- JSON object with proof
    request_sample TEXT,      -- HTTP request that triggered finding
    response_sample TEXT,     -- HTTP response excerpt

    -- References
    cwe_id TEXT,              -- CWE identifier
    owasp_category TEXT,      -- OWASP Top 10 category
    cvss_score REAL,
    reference_urls TEXT,      -- JSON array of reference URLs

    -- Metadata
    discovered_at INTEGER NOT NULL,
    verified_at INTEGER,
    false_positive BOOLEAN DEFAULT 0,
    notes TEXT,

    FOREIGN KEY(session_id) REFERENCES crawl_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_findings_session ON findings(session_id);
CREATE INDEX IF NOT EXISTS idx_findings_node ON findings(node_id);
CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
CREATE INDEX IF NOT EXISTS idx_findings_type ON findings(finding_type);
CREATE INDEX IF NOT EXISTS idx_findings_false_positive ON findings(false_positive);

-- Technology detection
CREATE TABLE IF NOT EXISTS technologies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL,

    category TEXT NOT NULL CHECK(category IN (
        'web_server',
        'application_server',
        'framework',
        'cms',
        'javascript_framework',
        'database',
        'cache',
        'cdn',
        'analytics',
        'authentication',
        'other'
    )),

    name TEXT NOT NULL,
    version TEXT,
    confidence INTEGER CHECK(confidence BETWEEN 0 AND 100),

    -- Detection method
    detection_method TEXT NOT NULL CHECK(detection_method IN (
        'header',
        'cookie',
        'html_pattern',
        'url_pattern',
        'response_pattern',
        'error_message',
        'file_hash'
    )),
    evidence TEXT,

    discovered_at INTEGER NOT NULL,

    FOREIGN KEY(node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_technologies_node ON technologies(node_id);
CREATE INDEX IF NOT EXISTS idx_technologies_category ON technologies(category);
CREATE INDEX IF NOT EXISTS idx_technologies_name ON technologies(name);

-- HTTP request/response log
CREATE TABLE IF NOT EXISTS http_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    node_id INTEGER,

    -- Request
    request_method TEXT NOT NULL,
    request_url TEXT NOT NULL,
    request_headers TEXT,
    request_body TEXT,

    -- Response
    response_code INTEGER NOT NULL,
    response_headers TEXT,
    response_body TEXT,
    response_time_ms INTEGER,
    response_size INTEGER,

    -- Metadata
    timestamp INTEGER NOT NULL,
    error TEXT,

    FOREIGN KEY(session_id) REFERENCES crawl_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(node_id) REFERENCES nodes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_http_transactions_session ON http_transactions(session_id);
CREATE INDEX IF NOT EXISTS idx_http_transactions_node ON http_transactions(node_id);
CREATE INDEX IF NOT EXISTS idx_http_transactions_timestamp ON http_transactions(timestamp);
            ",
        )?;
        Ok(())
    }

    // Session management
    pub fn create_session(&self, scan_type: &str, seed_urls: &str) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let timestamp = current_timestamp();

        self.conn.execute(
            "INSERT INTO crawl_sessions (id, start_time, status, scan_type, seed_urls) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&session_id, timestamp, "running", scan_type, seed_urls],
        )?;

        Ok(session_id)
    }

    pub fn complete_session(&self, session_id: &str) -> Result<()> {
        let timestamp = current_timestamp();
        self.conn.execute(
            "UPDATE crawl_sessions SET status = ?1, end_time = ?2 WHERE id = ?3",
            params!["completed", timestamp, session_id],
        )?;
        Ok(())
    }

    pub fn fail_session(&self, session_id: &str) -> Result<()> {
        let timestamp = current_timestamp();
        self.conn.execute(
            "UPDATE crawl_sessions SET status = ?1, end_time = ?2 WHERE id = ?3",
            params!["failed", timestamp, session_id],
        )?;
        Ok(())
    }

    // Map management
    pub fn create_map(&self, session_id: &str) -> Result<String> {
        let map_id = uuid::Uuid::new_v4().to_string();
        let timestamp = current_timestamp();

        self.conn.execute(
            "INSERT INTO maps (id, session_id, created_at) VALUES (?1, ?2, ?3)",
            params![&map_id, session_id, timestamp],
        )?;

        Ok(map_id)
    }

    // Node operations
    pub fn insert_node(&self, map_id: &str, node: &CrawlNode) -> Result<i64> {
        let timestamp = current_timestamp();
        let service_type_str = node.service_type.as_ref().map(|st| st.as_str());

        self.conn.execute(
            "INSERT INTO nodes (
                map_id, url, domain, node_type, status, depth, discovered_at,
                last_crawled, response_code, response_time_ms, content_type,
                content_length, title, forms_count, service_type, headers, body_sample
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                map_id,
                &node.url,
                &node.domain,
                "endpoint",
                "crawled",
                0,
                timestamp,
                timestamp,
                node.status_code,
                node.response_time_ms,
                &node.content_type,
                node.content_length.map(|l| l as i64),
                &node.title,
                node.forms_count as i64,
                service_type_str,
                &node.headers,
                &node.body_sample,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_node_by_url(&self, map_id: &str, url: &str) -> Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM nodes WHERE map_id = ?1 AND url = ?2")?;

        let result = stmt
            .query_row(params![map_id, url], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    // Finding operations
    pub fn insert_finding(&self, session_id: &str, finding: &Finding) -> Result<i64> {
        let timestamp = current_timestamp();

        self.conn.execute(
            "INSERT INTO findings (
                session_id, node_id, finding_type, severity, confidence,
                title, description, impact, remediation, evidence,
                cwe_id, owasp_category, discovered_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                session_id,
                finding.node_id,
                finding.finding_type.as_str(),
                finding.severity.as_str(),
                "likely", // default confidence
                &finding.title,
                &finding.description,
                &finding.impact,
                &finding.remediation,
                &finding.evidence,
                &finding.cwe_id,
                &finding.owasp_category,
                timestamp,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_findings_by_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<(i64, String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, severity, title, description FROM findings WHERE session_id = ?1 AND false_positive = 0 ORDER BY CASE severity
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'medium' THEN 3
                WHEN 'low' THEN 4
                WHEN 'info' THEN 5
            END, id"
        )?;

        let findings = stmt
            .query_map(params![session_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(findings)
    }

    pub fn get_findings_count_by_severity(&self, session_id: &str) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT severity, COUNT(*) FROM findings WHERE session_id = ?1 AND false_positive = 0 GROUP BY severity"
        )?;

        let counts = stmt
            .query_map(params![session_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        Ok(counts)
    }

    // Technology detection
    pub fn insert_technology(
        &self,
        node_id: i64,
        category: &str,
        name: &str,
        version: Option<&str>,
        detection_method: &str,
        evidence: Option<&str>,
        confidence: u8,
    ) -> Result<i64> {
        let timestamp = current_timestamp();

        self.conn.execute(
            "INSERT INTO technologies (node_id, category, name, version, detection_method, evidence, confidence, discovered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![node_id, category, name, version, detection_method, evidence, confidence, timestamp],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_technologies_by_node(
        &self,
        node_id: i64,
    ) -> Result<Vec<(String, String, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT category, name, version FROM technologies WHERE node_id = ?1")?;

        let techs = stmt
            .query_map(params![node_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(techs)
    }

    // HTTP transaction logging
    pub fn log_http_transaction(
        &self,
        session_id: &str,
        node_id: Option<i64>,
        method: &str,
        url: &str,
        request_headers: Option<&str>,
        response_code: u16,
        response_headers: Option<&str>,
        response_time_ms: Option<u64>,
    ) -> Result<i64> {
        let timestamp = current_timestamp();

        self.conn.execute(
            "INSERT INTO http_transactions (
                session_id, node_id, request_method, request_url, request_headers,
                response_code, response_headers, response_time_ms, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                node_id,
                method,
                url,
                request_headers,
                response_code as i64,
                response_headers,
                response_time_ms.map(|t| t as i64),
                timestamp,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    // Query methods
    pub fn get_nodes_by_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<(i64, String, i64, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.url, n.response_code, n.service_type
             FROM nodes n
             JOIN maps m ON n.map_id = m.id
             WHERE m.session_id = ?1",
        )?;

        let nodes = stmt
            .query_map(params![session_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(nodes)
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}
