use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

/// Security finding information for TUI display
#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub title: String,
    pub severity: String,
    pub description: String,
    pub impact: String,
    pub remediation: String,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
}

/// Message types for communication between crawler and TUI
#[derive(Debug, Clone)]
pub enum CrawlMessage {
    /// Session started with ID
    SessionStarted {
        session_id: String,
    },
    /// A URL was discovered/processed
    Finding {
        url: String,
        status_code: u16,
        content_type: Option<String>,
        security_findings: Vec<SecurityFinding>,
    },
    /// Progress update
    Progress {
        processed: usize,
        message: String,
    },
    /// Log message
    Log {
        level: LogLevel,
        message: String,
    },
    /// Crawl completed
    Complete {
        total: usize,
        findings_count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// TUI state for monitoring crawl progress
pub struct CrawlMonitor {
    findings: Vec<(String, u16, Option<String>, Vec<SecurityFinding>)>,  // (url, status_code, content_type, security_findings)
    selected_finding: Option<usize>,
    logs: Vec<(LogLevel, String)>,
    progress_count: usize,
    progress_message: String,
    session_id: Option<String>,
    is_complete: bool,
    scroll_findings: usize,
    scroll_logs: usize,
    rx: mpsc::UnboundedReceiver<CrawlMessage>,
}

impl CrawlMonitor {
    pub fn new(rx: mpsc::UnboundedReceiver<CrawlMessage>) -> Self {
        Self {
            findings: Vec::new(),
            selected_finding: None,
            logs: Vec::new(),
            progress_count: 0,
            progress_message: "Starting crawl...".to_string(),
            session_id: None,
            is_complete: false,
            scroll_findings: 0,
            scroll_logs: 0,
            rx,
        }
    }

    /// Process incoming messages from the crawler
    fn process_messages(&mut self) {
        // Process all available messages without blocking
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                CrawlMessage::SessionStarted { session_id } => {
                    self.session_id = Some(session_id);
                }
                CrawlMessage::Finding {
                    url,
                    status_code,
                    content_type,
                    security_findings,
                } => {
                    self.findings.push((url, status_code, content_type, security_findings));

                    // Keep only last 1000 findings to prevent memory issues
                    if self.findings.len() > 1000 {
                        self.findings.drain(0..self.findings.len() - 1000);
                        // Adjust selected index if needed
                        if let Some(selected) = self.selected_finding {
                            self.selected_finding = Some(selected.saturating_sub(self.findings.len() - 1000));
                        }
                    }
                }
                CrawlMessage::Progress { processed, message } => {
                    self.progress_count = processed;
                    self.progress_message = message;
                }
                CrawlMessage::Log { level, message } => {
                    self.logs.push((level, message));

                    // Keep only last 500 log entries
                    if self.logs.len() > 500 {
                        self.logs.drain(0..self.logs.len() - 500);
                    }
                }
                CrawlMessage::Complete { total, findings_count } => {
                    self.is_complete = true;
                    self.progress_count = total;
                    self.progress_message = format!(
                        "Crawl complete! {} URLs processed, {} findings",
                        total, findings_count
                    );
                }
            }
        }
    }

    fn render_findings(&self, f: &mut Frame, area: Rect) {
        let title = format!(" Findings ({}) ", self.findings.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let height = inner.height as usize;
        let total_items = self.findings.len();

        if total_items == 0 {
            let empty_msg = Paragraph::new("No findings yet... waiting for results")
                .style(Style::default().fg(Color::DarkGray))
                .wrap(Wrap { trim: true });
            f.render_widget(empty_msg, inner);
            return;
        }

        // Calculate scroll offset based on selection
        let scroll_offset = if let Some(selected) = self.selected_finding {
            // Ensure selected item is visible
            if selected < self.scroll_findings {
                // Selected item is above viewport, scroll up
                selected
            } else if selected >= self.scroll_findings + height {
                // Selected item is below viewport, scroll down
                selected.saturating_sub(height - 1)
            } else {
                // Selected item is visible, keep current scroll
                self.scroll_findings
            }
        } else {
            // No selection - auto-scroll to bottom if new items coming in
            if self.scroll_findings == 0 && total_items > height {
                total_items.saturating_sub(height)
            } else {
                self.scroll_findings.min(total_items.saturating_sub(height))
            }
        };

        let items: Vec<ListItem> = self
            .findings
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(height)
            .map(|(idx, (url, status_code, content_type, security_findings))| {
                let status_icon = match status_code {
                    200..=299 => "✓",
                    300..=399 => "→",
                    400..=499 => "!",
                    _ => "✗",
                };

                // Add security indicator if there are findings
                let security_badge = if !security_findings.is_empty() {
                    let max_severity = security_findings.iter()
                        .map(|f| match f.severity.as_str() {
                            "critical" => 4,
                            "high" => 3,
                            "medium" => 2,
                            "low" => 1,
                            _ => 0,
                        })
                        .max()
                        .unwrap_or(0);

                    match max_severity {
                        4 => " [CRITICAL]",
                        3 => " [HIGH]",
                        2 => " [MEDIUM]",
                        1 => " [LOW]",
                        _ => " [INFO]",
                    }
                } else {
                    ""
                };

                let text = if let Some(ct) = content_type {
                    format!("{} [{}] {}{} [{}]", status_icon, status_code, url, security_badge, ct)
                } else {
                    format!("{} [{}] {}{}", status_icon, status_code, url, security_badge)
                };

                // Colorize based on status code or security findings
                let color = if !security_findings.is_empty() {
                    // Highlight security findings with severity-based colors
                    let has_critical = security_findings.iter().any(|f| f.severity == "critical");
                    let has_high = security_findings.iter().any(|f| f.severity == "high");
                    let has_medium = security_findings.iter().any(|f| f.severity == "medium");

                    if has_critical {
                        Color::Magenta // Critical findings in magenta
                    } else if has_high {
                        Color::Red // High findings in red
                    } else if has_medium {
                        Color::Yellow // Medium findings in yellow
                    } else {
                        Color::Cyan // Low/Info findings in cyan
                    }
                } else {
                    // Normal status code coloring
                    match status_code {
                        200..=299 => Color::Green,
                        300..=399 => Color::Yellow,
                        400..=499 => Color::Red,
                        _ => Color::DarkGray,
                    }
                };

                let mut style = Style::default().fg(color);

                // Highlight selected item with different background
                if Some(idx) == self.selected_finding {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }

                // Make security findings stand out even more
                if !security_findings.is_empty() {
                    style = style.add_modifier(Modifier::BOLD);
                }

                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, inner);

        // Render scroll indicator if content is scrollable
        if total_items > height {
            self.render_scrollbar(f, area, total_items, height, scroll_offset);
        }
    }

    fn render_scrollbar(&self, f: &mut Frame, area: Rect, total_items: usize, visible_items: usize, scroll_offset: usize) {
        // Calculate scrollbar properties
        let scrollbar_height = area.height.saturating_sub(2) as usize; // -2 for borders
        if scrollbar_height == 0 {
            return;
        }

        // Calculate thumb size (proportional to visible/total ratio)
        let thumb_size = ((visible_items as f32 / total_items as f32) * scrollbar_height as f32)
            .max(1.0)
            .floor() as usize;

        // Calculate thumb position
        let scroll_ratio = scroll_offset as f32 / (total_items - visible_items) as f32;
        let thumb_position = (scroll_ratio * (scrollbar_height - thumb_size) as f32)
            .floor() as usize;

        // Draw scrollbar on the right edge of the findings panel
        let scrollbar_x = area.x + area.width - 1;
        let scrollbar_start_y = area.y + 1; // +1 for top border

        for i in 0..scrollbar_height {
            let y = scrollbar_start_y + i as u16;
            let symbol = if i >= thumb_position && i < thumb_position + thumb_size {
                "█" // Solid block for thumb
            } else {
                "│" // Light vertical line for track
            };

            let style = if i >= thumb_position && i < thumb_position + thumb_size {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            f.render_widget(
                Paragraph::new(symbol).style(style),
                Rect {
                    x: scrollbar_x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }

    fn render_progress(&self, f: &mut Frame, area: Rect) {
        let (title, border_color) = if self.is_complete {
            (" Complete ", Color::Green)
        } else {
            (" Progress ", Color::Yellow)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let status_icon = if self.is_complete { "✓" } else { "⠋" };

        let mut text = vec![
            Line::from(vec![
                Span::styled(status_icon, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(
                    format!("{} URLs", self.progress_count),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ];

        // Show session ID if available
        if let Some(ref session_id) = self.session_id {
            text.push(Line::from(vec![
                Span::styled("Session: ", Style::default().fg(Color::DarkGray)),
                Span::styled(session_id.clone(), Style::default().fg(Color::Cyan)),
            ]));
            text.push(Line::from(""));
        }

        text.push(Line::from(self.progress_message.clone()));

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);
    }

    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Logs ")
            .border_style(Style::default().fg(Color::Magenta));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let height = inner.height as usize;
        let total_items = self.logs.len();

        // Auto-scroll to bottom if not manually scrolled
        let scroll_offset = if self.scroll_logs == 0 && total_items > height {
            total_items.saturating_sub(height)
        } else {
            self.scroll_logs.min(total_items.saturating_sub(height))
        };

        let items: Vec<ListItem> = self
            .logs
            .iter()
            .skip(scroll_offset)
            .take(height)
            .map(|(level, message)| {
                let (prefix, style) = match level {
                    LogLevel::Info => ("INFO ", Style::default().fg(Color::Blue)),
                    LogLevel::Warn => ("WARN ", Style::default().fg(Color::Yellow)),
                    LogLevel::Error => ("ERROR", Style::default().fg(Color::Red)),
                };
                ListItem::new(format!("[{}] {}", prefix, message)).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, inner);
    }

    fn render_hints(&self, f: &mut Frame, area: Rect) {
        let hints = if self.is_complete {
            Line::from(vec![
                Span::styled(" q/ESC ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Exit  "),
                Span::styled(" ↑/↓ ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Select  "),
                Span::styled(" PgUp/PgDn ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Scroll  "),
                Span::styled(" Home/End ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Top/Bottom  "),
                Span::styled(" Enter ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Details"),
            ])
        } else {
            Line::from(vec![
                Span::styled(" Ctrl+C ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Stop  "),
                Span::styled(" ↑/↓ ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Select  "),
                Span::styled(" PgUp/PgDn ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Scroll  "),
                Span::styled(" Home/End ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Top/Bottom  "),
                Span::styled(" Enter ", Style::default().fg(Color::Black).bg(Color::Gray)),
                Span::raw(" Details"),
            ])
        };

        let paragraph = Paragraph::new(hints)
            .style(Style::default().bg(Color::Black).fg(Color::Gray));
        f.render_widget(paragraph, area);
    }
}

/// Run the crawl monitor TUI (blocking function, should be run in separate thread)
pub fn run_monitor(
    rx: mpsc::UnboundedReceiver<CrawlMessage>,
    should_exit: Arc<AtomicBool>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut monitor = CrawlMonitor::new(rx);

    // Main loop
    loop {
        // Process any pending messages
        monitor.process_messages();

        // Draw UI
        terminal.draw(|f| {
            let size = f.area();

            // Split vertically: main area + hints bar
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),   // Main area
                    Constraint::Length(1), // Hints bar
                ])
                .split(size);

            // Split main area into left (findings) and right (progress + logs)
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60), // Findings (left)
                    Constraint::Percentage(40), // Progress + Logs (right)
                ])
                .split(vertical_chunks[0]);

            // Split right side into top (progress) and bottom (logs)
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8), // Progress (top right) - increased for session ID
                    Constraint::Min(10),   // Logs (bottom right)
                ])
                .split(main_chunks[1]);

            // Render panels
            monitor.render_findings(f, main_chunks[0]);
            monitor.render_progress(f, right_chunks[0]);
            monitor.render_logs(f, right_chunks[1]);
            monitor.render_hints(f, vertical_chunks[1]);
        })?;

        // Check for exit signal (but don't auto-exit on completion)
        if should_exit.load(Ordering::Relaxed) {
            break;
        }

        // Poll for keyboard events (non-blocking with timeout)
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Ctrl+C pressed - exit immediately
                    break;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break;
                }
                KeyCode::Up => {
                    if !monitor.findings.is_empty() {
                        if let Some(selected) = monitor.selected_finding {
                            let new_selected = selected.saturating_sub(1);
                            monitor.selected_finding = Some(new_selected);
                            // Update scroll to keep selection in view
                            if new_selected < monitor.scroll_findings {
                                monitor.scroll_findings = new_selected;
                            }
                        } else {
                            // Start selection at the last item
                            monitor.selected_finding = Some(monitor.findings.len().saturating_sub(1));
                        }
                    }
                }
                KeyCode::Down => {
                    if !monitor.findings.is_empty() {
                        if let Some(selected) = monitor.selected_finding {
                            let new_selected = (selected + 1).min(monitor.findings.len() - 1);
                            monitor.selected_finding = Some(new_selected);
                            // Scroll calculation is done in render_findings, no need to update here
                        } else {
                            // Start selection at the first item (top of viewport)
                            monitor.selected_finding = Some(monitor.scroll_findings);
                        }
                    }
                }
                KeyCode::Enter => {
                    // Show detailed view of selected finding
                    if let Some(selected) = monitor.selected_finding {
                        if let Some((url, status_code, content_type, security_findings)) = monitor.findings.get(selected) {
                            // Clear previous details and add separator
                            monitor.logs.push((LogLevel::Info, "".to_string()));
                            monitor.logs.push((LogLevel::Info,
                                "╔══════════════════════════════════════════════════════════╗".to_string()));
                            monitor.logs.push((LogLevel::Info,
                                "║                    FINDING DETAILS                       ║".to_string()));
                            monitor.logs.push((LogLevel::Info,
                                "╚══════════════════════════════════════════════════════════╝".to_string()));

                            // Basic info
                            monitor.logs.push((LogLevel::Info, format!("URL: {}", url)));
                            monitor.logs.push((LogLevel::Info, format!("Status Code: {}", status_code)));
                            monitor.logs.push((LogLevel::Info, format!(
                                "Content-Type: {}",
                                content_type.as_deref().unwrap_or("N/A")
                            )));

                            // Security findings if present
                            if !security_findings.is_empty() {
                                monitor.logs.push((LogLevel::Info, "".to_string()));
                                monitor.logs.push((LogLevel::Warn,
                                    "╔══════════════════════════════════════════════════════════╗".to_string()));
                                monitor.logs.push((LogLevel::Warn,
                                    "║                  SECURITY FINDINGS                       ║".to_string()));
                                monitor.logs.push((LogLevel::Warn,
                                    "╚══════════════════════════════════════════════════════════╝".to_string()));

                                for (i, finding) in security_findings.iter().enumerate() {
                                    let level = match finding.severity.as_str() {
                                        "critical" | "high" => LogLevel::Error,
                                        "medium" => LogLevel::Warn,
                                        _ => LogLevel::Info,
                                    };

                                    monitor.logs.push((LogLevel::Info, "".to_string()));
                                    monitor.logs.push((level, format!("[{}] {}", i + 1, finding.title)));
                                    monitor.logs.push((level, format!("  Severity: {}", finding.severity.to_uppercase())));

                                    if let Some(ref cwe) = finding.cwe {
                                        monitor.logs.push((LogLevel::Info, format!("  CWE: {}", cwe)));
                                    }
                                    if let Some(ref owasp) = finding.owasp {
                                        monitor.logs.push((LogLevel::Info, format!("  OWASP: {}", owasp)));
                                    }

                                    monitor.logs.push((LogLevel::Info, format!("  Description: {}", finding.description)));
                                    monitor.logs.push((LogLevel::Info, format!("  Impact: {}", finding.impact)));
                                    monitor.logs.push((LogLevel::Info, format!("  Remediation: {}", finding.remediation)));
                                }
                            }

                            monitor.logs.push((LogLevel::Info, "".to_string()));
                            monitor.logs.push((LogLevel::Info,
                                "══════════════════════════════════════════════════════════".to_string()));
                        }
                    }
                }
                KeyCode::PageUp => {
                    if !monitor.findings.is_empty() {
                        let height = 10; // Approximate page size
                        monitor.scroll_findings = monitor.scroll_findings.saturating_sub(height);
                        // Update selection to stay in view
                        if let Some(selected) = monitor.selected_finding {
                            if selected >= monitor.scroll_findings + height {
                                monitor.selected_finding = Some(monitor.scroll_findings + height - 1);
                            }
                        }
                    }
                }
                KeyCode::PageDown => {
                    if !monitor.findings.is_empty() {
                        let height = 10; // Approximate page size
                        let max_scroll = monitor.findings.len().saturating_sub(height);
                        monitor.scroll_findings = (monitor.scroll_findings + height).min(max_scroll);
                        // Update selection to stay in view
                        if let Some(selected) = monitor.selected_finding {
                            if selected < monitor.scroll_findings {
                                monitor.selected_finding = Some(monitor.scroll_findings);
                            }
                        }
                    }
                }
                KeyCode::Home => {
                    // Jump to top
                    monitor.scroll_findings = 0;
                    monitor.selected_finding = Some(0);
                }
                KeyCode::End => {
                    // Jump to bottom
                    if !monitor.findings.is_empty() {
                        monitor.selected_finding = Some(monitor.findings.len() - 1);
                        monitor.scroll_findings = monitor.findings.len().saturating_sub(10);
                    }
                }
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Create a channel pair for crawl monitoring
pub fn create_monitor_channel() -> (mpsc::UnboundedSender<CrawlMessage>, mpsc::UnboundedReceiver<CrawlMessage>) {
    mpsc::unbounded_channel()
}
