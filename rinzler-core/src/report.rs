// Report generation from database

use crate::data::Database;
use rusqlite::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Text,
    Json,
    Csv,
    Html,
    Markdown,
}

impl ReportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(ReportFormat::Text),
            "json" => Some(ReportFormat::Json),
            "csv" => Some(ReportFormat::Csv),
            "html" => Some(ReportFormat::Html),
            "markdown" | "md" => Some(ReportFormat::Markdown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub session_id: String,
    pub total_nodes: usize,
    pub findings: Vec<FindingData>,
    pub severity_counts: SeverityCounts,
    pub scan_info: ScanInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sitemap_nodes: Option<Vec<SitemapNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitemapNode {
    pub url: String,
    pub status_code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingData {
    pub id: i64,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub finding_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owasp_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub info: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInfo {
    pub start_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    pub status: String,
    pub seed_urls: String,
}

pub fn gather_report_data(db: &Database, session_id: &str, include_sitemap: bool) -> Result<ReportData> {
    // Get session info
    let scan_info = {
        let conn = db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT start_time, end_time, status, seed_urls FROM crawl_sessions WHERE id = ?1"
        )?;

        stmt.query_row([session_id], |row| {
            Ok(ScanInfo {
                start_time: row.get(0)?,
                end_time: row.get(1)?,
                status: row.get(2)?,
                seed_urls: row.get(3)?,
            })
        })?
    };

    // Get node count
    let nodes = db.get_nodes_by_session(session_id)?;
    let total_nodes = nodes.len();

    // Get severity counts
    let severity_counts_raw = db.get_findings_count_by_severity(session_id)?;
    let mut severity_counts = SeverityCounts {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        info: 0,
    };

    for (severity, count) in severity_counts_raw {
        match severity.as_str() {
            "critical" => severity_counts.critical = count,
            "high" => severity_counts.high = count,
            "medium" => severity_counts.medium = count,
            "low" => severity_counts.low = count,
            "info" => severity_counts.info = count,
            _ => {}
        }
    }

    // Get detailed findings
    let conn = db.get_connection();
    let mut stmt = conn.prepare(
        "SELECT f.id, f.severity, f.title, f.description, n.url, f.finding_type,
                f.cwe_id, f.owasp_category, f.impact, f.remediation
         FROM findings f
         JOIN nodes n ON f.node_id = n.id
         WHERE f.session_id = ?1 AND f.false_positive = 0
         ORDER BY CASE f.severity
             WHEN 'critical' THEN 1
             WHEN 'high' THEN 2
             WHEN 'medium' THEN 3
             WHEN 'low' THEN 4
             WHEN 'info' THEN 5
         END, f.id"
    )?;

    let findings = stmt.query_map([session_id], |row| {
        Ok(FindingData {
            id: row.get(0)?,
            severity: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            url: row.get(4)?,
            finding_type: row.get(5)?,
            cwe_id: row.get(6)?,
            owasp_category: row.get(7)?,
            impact: row.get(8)?,
            remediation: row.get(9)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;

    // Optionally gather sitemap data
    let sitemap_nodes = if include_sitemap {
        let conn = db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT n.url, n.response_code, n.content_type
             FROM nodes n
             JOIN maps m ON n.map_id = m.id
             WHERE m.session_id = ?1
             ORDER BY n.url"
        )?;

        let nodes = stmt.query_map([session_id], |row| {
            Ok(SitemapNode {
                url: row.get(0)?,
                status_code: row.get::<_, Option<u16>>(1)?.unwrap_or(0),
                content_type: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Some(nodes)
    } else {
        None
    };

    Ok(ReportData {
        session_id: session_id.to_string(),
        total_nodes,
        findings,
        severity_counts,
        scan_info,
        sitemap_nodes,
    })
}

pub fn generate_text_report(data: &ReportData) -> String {
    let mut report = String::new();

    // Header
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    report.push_str("                        RINZLER SECURITY SCAN REPORT\n");
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    // Session info
    report.push_str(&format!("Session ID:   {}\n", data.session_id));
    report.push_str(&format!("Status:       {}\n", data.status_to_string()));
    report.push_str(&format!("Scan Date:    {}\n", data.format_timestamp(data.scan_info.start_time)));

    if let Some(end_time) = data.scan_info.end_time {
        let duration = end_time - data.scan_info.start_time;
        report.push_str(&format!("Duration:     {} seconds\n", duration));
    }

    report.push_str(&format!("Targets:      {}\n", data.format_targets()));
    report.push_str(&format!("Pages Found:  {}\n", data.total_nodes));
    report.push_str("\n");

    // Include sitemap if present
    if let Some(ref sitemap_nodes) = data.sitemap_nodes {
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        report.push_str("SITE MAP\n");
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
        report.push_str(&generate_sitemap_tree(sitemap_nodes));
        report.push_str("\n");
    }

    // Executive Summary
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    report.push_str("EXECUTIVE SUMMARY\n");
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    let total_findings = data.severity_counts.critical
        + data.severity_counts.high
        + data.severity_counts.medium
        + data.severity_counts.low
        + data.severity_counts.info;

    report.push_str(&format!("Total Findings: {}\n\n", total_findings));

    if data.severity_counts.critical > 0 {
        report.push_str(&format!("  [CRITICAL] {}  (Immediate action required)\n", data.severity_counts.critical));
    }
    if data.severity_counts.high > 0 {
        report.push_str(&format!("  [HIGH]     {}  (High priority)\n", data.severity_counts.high));
    }
    if data.severity_counts.medium > 0 {
        report.push_str(&format!("  [MEDIUM]   {}  (Should be addressed)\n", data.severity_counts.medium));
    }
    if data.severity_counts.low > 0 {
        report.push_str(&format!("  [LOW]      {}  (Minor issues)\n", data.severity_counts.low));
    }
    if data.severity_counts.info > 0 {
        report.push_str(&format!("  [INFO]     {}  (Informational)\n", data.severity_counts.info));
    }
    report.push_str("\n");

    // Detailed findings
    if !data.findings.is_empty() {
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        report.push_str("DETAILED FINDINGS\n");
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        for (idx, finding) in data.findings.iter().enumerate() {
            report.push_str(&format!("[{}] {}\n", idx + 1, finding.title));
            report.push_str(&format!("Severity:     {}\n", finding.severity.to_uppercase()));
            report.push_str(&format!("Type:         {}\n", format_finding_type(&finding.finding_type)));
            report.push_str(&format!("URL:          {}\n", finding.url));

            if let Some(ref cwe) = finding.cwe_id {
                report.push_str(&format!("CWE:          {}\n", cwe));
            }
            if let Some(ref owasp) = finding.owasp_category {
                report.push_str(&format!("OWASP:        {}\n", owasp));
            }

            report.push_str("\nDescription:\n");
            report.push_str(&wrap_text(&finding.description, 80, "  "));
            report.push_str("\n\n");

            if let Some(ref impact) = finding.impact {
                report.push_str("Impact:\n");
                report.push_str(&wrap_text(impact, 80, "  "));
                report.push_str("\n\n");
            }

            if let Some(ref remediation) = finding.remediation {
                report.push_str("Remediation:\n");
                report.push_str(&wrap_text(remediation, 80, "  "));
                report.push_str("\n\n");
            }

            report.push_str("────────────────────────────────────────────────────────────────────────────────\n\n");
        }
    }

    // Footer
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    report.push_str("                          End of Report\n");
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    report.push_str("\nGenerated by Rinzler - A somewhat intelligent Web API scanner\n");
    report.push_str("For authorized security testing only.\n\n");

    report
}

pub fn generate_json_report(data: &ReportData) -> Result<String, serde_json::Error> {
    // Create a structured JSON report with enhanced metadata
    let json_report = serde_json::json!({
        "report": {
            "metadata": {
                "generator": "Rinzler",
                "version": env!("CARGO_PKG_VERSION"),
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "format": "json",
                "disclaimer": "For authorized security testing only"
            },
            "session": {
                "id": data.session_id,
                "status": data.scan_info.status,
                "start_time": format_iso8601_timestamp(data.scan_info.start_time),
                "end_time": data.scan_info.end_time.map(format_iso8601_timestamp),
                "duration_seconds": data.scan_info.end_time.map(|end| end - data.scan_info.start_time),
                "targets": parse_targets(&data.scan_info.seed_urls)
            },
            "summary": {
                "total_pages": data.total_nodes,
                "total_findings": data.severity_counts.critical
                    + data.severity_counts.high
                    + data.severity_counts.medium
                    + data.severity_counts.low
                    + data.severity_counts.info,
                "severity_breakdown": {
                    "critical": data.severity_counts.critical,
                    "high": data.severity_counts.high,
                    "medium": data.severity_counts.medium,
                    "low": data.severity_counts.low,
                    "info": data.severity_counts.info
                }
            },
            "findings": data.findings,
            "sitemap": data.sitemap_nodes.as_ref().map(|nodes| {
                serde_json::json!({
                    "total_nodes": nodes.len(),
                    "nodes": nodes
                })
            })
        }
    });

    serde_json::to_string_pretty(&json_report)
}

pub fn save_report(content: &str, path: &Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Helper functions
impl ReportData {
    fn status_to_string(&self) -> &str {
        match self.scan_info.status.as_str() {
            "completed" => "Completed",
            "failed" => "Failed",
            "running" => "Running",
            "cancelled" => "Cancelled",
            _ => "Unknown",
        }
    }

    fn format_timestamp(&self, timestamp: i64) -> String {
        use chrono::{DateTime, Utc};
        let datetime = DateTime::<Utc>::from_timestamp(timestamp, 0)
            .unwrap_or_else(|| Utc::now());
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    fn format_targets(&self) -> String {
        // Parse JSON seed_urls
        if let Ok(urls) = serde_json::from_str::<Vec<String>>(&self.scan_info.seed_urls) {
            if urls.len() == 1 {
                urls[0].clone()
            } else {
                format!("{} URLs", urls.len())
            }
        } else {
            "Unknown".to_string()
        }
    }
}

fn format_finding_type(finding_type: &str) -> String {
    finding_type.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap_text(text: &str, width: usize, indent: &str) -> String {
    let mut result = String::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width - indent.len() {
            if !current_line.is_empty() {
                result.push_str(indent);
                result.push_str(&current_line);
                result.push('\n');
                current_line.clear();
            }
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        result.push_str(indent);
        result.push_str(&current_line);
        result.push('\n');
    }

    result
}

fn generate_sitemap_tree(nodes: &[SitemapNode]) -> String {
    use std::collections::HashMap;

    if nodes.is_empty() {
        return "  (empty)\n".to_string();
    }

    // Build a tree structure from URLs
    let mut tree: HashMap<String, Vec<(String, &SitemapNode)>> = HashMap::new();

    for node in nodes {
        if let Ok(parsed) = url::Url::parse(&node.url) {
            let domain = parsed.host_str().unwrap_or("unknown").to_string();
            let path = parsed.path().to_string();

            // Split path into segments
            let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

            // Build hierarchical path
            let mut current_path = domain.clone();
            tree.entry(current_path.clone()).or_default();

            for (i, segment) in segments.iter().enumerate() {
                let parent_path = current_path.clone();
                current_path = format!("{}/{}", current_path, segment);

                // Only add the leaf node with metadata
                if i == segments.len() - 1 {
                    tree.entry(parent_path)
                        .or_default()
                        .push((current_path.clone(), node));
                } else {
                    tree.entry(parent_path)
                        .or_default()
                        .push((current_path.clone(), node));
                    tree.entry(current_path.clone()).or_default();
                }
            }

            // If root path, add it directly to domain
            if segments.is_empty() {
                tree.entry(domain.clone())
                    .or_default()
                    .push((domain.clone(), node));
            }
        }
    }

    // Simple flat list representation for now (tree structure is complex)
    let mut result = String::new();

    for (i, node) in nodes.iter().enumerate() {
        let is_last = i == nodes.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };

        // Extract path from URL
        let display_url = if let Ok(parsed) = url::Url::parse(&node.url) {
            let host = parsed.host_str().unwrap_or("unknown");
            let path = parsed.path();
            if i == 0 {
                format!("{}{}", host, path)
            } else {
                // Check if same host as previous
                let prev_host = url::Url::parse(&nodes[i-1].url)
                    .ok()
                    .and_then(|u| u.host_str().map(String::from));
                if prev_host.as_deref() == Some(host) {
                    format!("    {}", path)
                } else {
                    format!("{}{}", host, path)
                }
            }
        } else {
            node.url.clone()
        };

        // Format status code with color indicator
        let status_indicator = match node.status_code {
            200..=299 => "✓",
            300..=399 => "→",
            400..=499 => "⚠",
            500..=599 => "✗",
            _ => "?",
        };

        let content_type_short = node.content_type.as_ref()
            .and_then(|ct| ct.split(';').next())
            .and_then(|ct| ct.split('/').nth(1))
            .unwrap_or("?");

        result.push_str(&format!("{}{}  [{} {}] {}\n",
            prefix, display_url, status_indicator, node.status_code, content_type_short));
    }

    result
}

fn format_iso8601_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    let datetime = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| Utc::now());
    datetime.to_rfc3339()
}

fn parse_targets(seed_urls_json: &str) -> serde_json::Value {
    serde_json::from_str::<Vec<String>>(seed_urls_json)
        .map(|urls| serde_json::json!(urls))
        .unwrap_or_else(|_| serde_json::json!([]))
}
