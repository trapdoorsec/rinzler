use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExitMode {
    None,
    Normal,      // exit, quit - ask about saving
    ForceQuit,   // :q! - don't save, don't ask
    WriteQuit,   // :wq!, ZZ - save and quit, don't ask
}

pub struct App {
    input: String,
    history: Vec<String>,
    output: Vec<String>,
    cursor_position: usize,
    should_quit: bool,
    scroll_offset: usize,
    history_index: Option<usize>,
    temp_input: String,
    exit_mode: ExitMode,
    awaiting_save_confirmation: bool,
}

impl App {
    pub fn new() -> Self {
        let banner = r#"
    ╔════════════════════════════════════════════════════════╗
    ║  ██████╗ ██╗███╗   ██╗███████╗██╗     ███████╗██████╗  ║
    ║  ██╔══██╗██║████╗  ██║╚══███╔╝██║     ██╔════╝██╔══██╗ ║
    ║  ██████╔╝██║██╔██╗ ██║  ███╔╝ ██║     █████╗  ██████╔╝ ║
    ║  ██╔══██╗██║██║╚██╗██║ ███╔╝  ██║     ██╔══╝  ██╔══██╗ ║
    ║  ██║  ██║██║██║ ╚████║███████╗███████╗███████╗██║  ██║ ║
    ║  ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝ ║
    ║                                                        ║
    ║           ⚡ Interactive REPL Interface ⚡             ║
    ╚════════════════════════════════════════════════════════╝
        "#;

        let mut output = Vec::new();
        for line in banner.lines() {
            output.push(line.to_string());
        }
        output.push(String::new());
        output.push("  Type 'help' for available commands, 'exit' or 'quit' to exit.".to_string());
        output.push(String::new());

        Self {
            input: String::new(),
            history: Vec::new(),
            output,
            cursor_position: 0,
            should_quit: false,
            scroll_offset: 0,
            history_index: None,
            temp_input: String::new(),
            exit_mode: ExitMode::None,
            awaiting_save_confirmation: false,
        }
    }

    pub fn add_output(&mut self, message: impl Into<String>) {
        self.output.push(message.into());
        // Keep only last 1000 lines to prevent memory issues
        if self.output.len() > 1000 {
            self.output.drain(0..self.output.len() - 1000);
        }
        // Reset scroll to auto-scroll to bottom on new output
        self.scroll_offset = 0;
    }

    pub fn navigate_history_backward(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // If starting history navigation, save current input
        if self.history_index.is_none() {
            self.temp_input = self.input.clone();
        }

        let new_index = match self.history_index {
            None => Some(self.history.len() - 1),
            Some(0) => Some(0), // Already at oldest
            Some(idx) => Some(idx - 1),
        };

        if let Some(idx) = new_index {
            self.history_index = new_index;
            self.input = self.history[idx].clone();
            self.cursor_position = self.input.len();
        }
    }

    pub fn navigate_history_forward(&mut self) {
        if self.history.is_empty() || self.history_index.is_none() {
            return;
        }

        let new_index = match self.history_index {
            Some(idx) if idx >= self.history.len() - 1 => {
                // Reached the end, restore temp input
                self.input = self.temp_input.clone();
                self.cursor_position = self.input.len();
                self.history_index = None;
                self.temp_input.clear();
                return;
            }
            Some(idx) => Some(idx + 1),
            None => None,
        };

        if let Some(idx) = new_index {
            self.history_index = new_index;
            self.input = self.history[idx].clone();
            self.cursor_position = self.input.len();
        }
    }

    fn get_history_file_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".rinzler_history")
    }

    pub fn load_history(&mut self) {
        let path = Self::get_history_file_path();
        if let Ok(content) = fs::read_to_string(&path) {
            let mut lines: Vec<String> = content
                .lines()
                .map(|s| s.to_string())
                .collect();

            // Keep only the last 100 entries
            if lines.len() > 100 {
                lines.drain(0..lines.len() - 100);
            }

            self.history = lines;
        }
    }

    pub fn save_history(&self) -> Result<()> {
        let path = Self::get_history_file_path();
        let content = self.history.join("\n");
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn request_exit(&mut self, mode: ExitMode) {
        self.exit_mode = mode;

        match mode {
            ExitMode::Normal => {
                // Ask user if they want to save
                if !self.history.is_empty() {
                    self.awaiting_save_confirmation = true;
                    self.add_output("");
                    self.add_output("Save command history to ~/.rinzler_history? [y/N]:");
                } else {
                    self.should_quit = true;
                }
            }
            ExitMode::ForceQuit => {
                // Just quit, don't save
                self.should_quit = true;
            }
            ExitMode::WriteQuit => {
                // Save and quit
                if !self.history.is_empty() {
                    if let Err(e) = self.save_history() {
                        self.add_output(format!("Error saving history: {}", e));
                    } else {
                        self.add_output("History saved to ~/.rinzler_history");
                    }
                }
                self.should_quit = true;
            }
            ExitMode::None => {}
        }
    }

    pub fn handle_save_confirmation(&mut self, response: &str) {
        self.awaiting_save_confirmation = false;

        let response = response.trim().to_lowercase();
        if response == "y" || response == "yes" {
            if let Err(e) = self.save_history() {
                self.add_output(format!("Error saving history: {}", e));
            } else {
                self.add_output("History saved to ~/.rinzler_history");
            }
        } else {
            self.add_output("History not saved.");
        }
        self.should_quit = true;
    }

    pub fn handle_input(&mut self, input: String) {
        if input.is_empty() {
            return;
        }

        // If awaiting save confirmation, handle it separately
        if self.awaiting_save_confirmation {
            self.handle_save_confirmation(&input);
            return;
        }

        // Add command to history and limit to 100 items
        self.history.push(input.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        // Reset history navigation
        self.history_index = None;
        self.temp_input.clear();

        self.add_output(format!("> {}", input));

        // Parse and execute command
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "exit" | "quit" => {
                self.request_exit(ExitMode::Normal);
            }
            ":q!" => {
                self.request_exit(ExitMode::ForceQuit);
            }
            ":wq!" | "ZZ" => {
                self.request_exit(ExitMode::WriteQuit);
            }
            "help" => {
                self.add_output("Available commands:");
                self.add_output("  init [path]                    - Initialize Rinzler configuration");
                self.add_output("  workspace create <name>        - Create a new workspace");
                self.add_output("  workspace remove <name>        - Remove a workspace");
                self.add_output("  workspace list                 - List all workspaces");
                self.add_output("  workspace rename <old> <new>   - Rename a workspace");
                self.add_output("  crawl <url> [threads]          - Passively crawl a URL");
                self.add_output("  fuzz <url> [wordlist] [threads] - Actively fuzz a URL");
                self.add_output("  plugin list                    - List all plugins");
                self.add_output("  plugin register <file> <name>  - Register a plugin");
                self.add_output("  plugin unregister <name>       - Unregister a plugin");
                self.add_output("  clear                          - Clear the output");
                self.add_output("  help                           - Show this help message");
                self.add_output("  exit, quit                     - Exit the REPL");
            }
            "clear" => {
                self.output.clear();
            }
            "init" => {
                let path = parts.get(1).unwrap_or(&"~/.config/rinzler/database");
                self.add_output(format!("Initializing Rinzler at: {}", path));
                self.add_output("Note: Interactive prompts not yet supported in TUI mode");
                self.add_output("Use CLI mode for initial setup: rinzler init");
            }
            "workspace" => {
                if parts.len() < 2 {
                    self.add_output("Error: workspace command requires a subcommand");
                    self.add_output("Try: workspace create|remove|list|rename");
                    return;
                }
                match parts[1] {
                    "create" => {
                        if let Some(name) = parts.get(2) {
                            self.add_output(format!("Creating workspace: {}", name));
                            self.add_output("TODO: Implement workspace creation");
                        } else {
                            self.add_output("Error: workspace create requires a name");
                        }
                    }
                    "remove" => {
                        if let Some(name) = parts.get(2) {
                            self.add_output(format!("Removing workspace: {}", name));
                            self.add_output("TODO: Implement workspace removal");
                        } else {
                            self.add_output("Error: workspace remove requires a name");
                        }
                    }
                    "list" => {
                        self.add_output("Listing workspaces:");
                        self.add_output("TODO: Implement workspace listing");
                    }
                    "rename" => {
                        if parts.len() >= 4 {
                            self.add_output(format!(
                                "Renaming workspace from '{}' to '{}'",
                                parts[2], parts[3]
                            ));
                            self.add_output("TODO: Implement workspace renaming");
                        } else {
                            self.add_output("Error: workspace rename requires old and new names");
                        }
                    }
                    _ => {
                        self.add_output(format!("Error: unknown workspace subcommand: {}", parts[1]));
                    }
                }
            }
            "crawl" => {
                if let Some(url) = parts.get(1) {
                    let threads = parts.get(2).unwrap_or(&"10");
                    self.add_output(format!("Crawling URL: {} with {} threads", url, threads));
                    self.add_output("TODO: Implement crawling logic");
                } else {
                    self.add_output("Error: crawl requires a URL");
                }
            }
            "fuzz" => {
                if let Some(url) = parts.get(1) {
                    let wordlist = parts.get(2).unwrap_or(&"~/.config/rinzler/wordlists/default.txt");
                    let threads = parts.get(3).unwrap_or(&"10");
                    self.add_output(format!("Fuzzing URL: {}", url));
                    self.add_output(format!("  Wordlist: {}", wordlist));
                    self.add_output(format!("  Threads: {}", threads));
                    self.add_output("TODO: Implement fuzzing logic");
                } else {
                    self.add_output("Error: fuzz requires a URL");
                }
            }
            "plugin" => {
                if parts.len() < 2 {
                    self.add_output("Error: plugin command requires a subcommand");
                    self.add_output("Try: plugin list|register|unregister");
                    return;
                }
                match parts[1] {
                    "list" => {
                        self.add_output("Listing plugins:");
                        self.add_output("TODO: Implement plugin listing");
                    }
                    "register" => {
                        if parts.len() >= 4 {
                            self.add_output(format!(
                                "Registering plugin '{}' from file: {}",
                                parts[3], parts[2]
                            ));
                            self.add_output("TODO: Implement plugin registration");
                        } else {
                            self.add_output("Error: plugin register requires file path and name");
                        }
                    }
                    "unregister" => {
                        if let Some(name) = parts.get(2) {
                            self.add_output(format!("Unregistering plugin: {}", name));
                            self.add_output("TODO: Implement plugin unregistration");
                        } else {
                            self.add_output("Error: plugin unregister requires a name");
                        }
                    }
                    _ => {
                        self.add_output(format!("Error: unknown plugin subcommand: {}", parts[1]));
                    }
                }
            }
            _ => {
                self.add_output(format!("Unknown command: {}", parts[0]));
                self.add_output("Type 'help' for available commands");
            }
        }
    }
}

pub fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Load command history from file
    app.load_history();

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Only process KeyPress events, ignore KeyRelease
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char(c) => {
                    app.input.insert(app.cursor_position, c);
                    app.cursor_position += 1;
                    // Reset history navigation when typing
                    app.history_index = None;
                    app.temp_input.clear();
                }
                KeyCode::Backspace => {
                    if app.cursor_position > 0 {
                        app.input.remove(app.cursor_position - 1);
                        app.cursor_position -= 1;
                        // Reset history navigation when editing
                        app.history_index = None;
                        app.temp_input.clear();
                    }
                }
                KeyCode::Enter => {
                    let input = app.input.drain(..).collect();
                    app.cursor_position = 0;
                    app.handle_input(input);
                }
                KeyCode::Up => {
                    app.navigate_history_backward();
                }
                KeyCode::Down => {
                    app.navigate_history_forward();
                }
                KeyCode::Left => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.cursor_position < app.input.len() {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Home => {
                    app.cursor_position = 0;
                }
                KeyCode::End => {
                    app.cursor_position = app.input.len();
                }
                KeyCode::Esc => {
                    app.should_quit = true;
                }
                KeyCode::PageUp => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    app.scroll_offset = (app.scroll_offset + 10).min(app.output.len().saturating_sub(1));
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),      // Output area
            Constraint::Length(1),   // Horizontal rule
            Constraint::Length(1),   // Input area
            Constraint::Length(1),   // Horizontal rule
            Constraint::Length(1),   // Status bar
        ])
        .split(f.area());

    // Output area - scrollable
    let output_height = chunks[0].height as usize;
    let total_lines = app.output.len();

    // Auto-scroll to bottom if not manually scrolled
    let scroll_offset = if app.scroll_offset == 0 && total_lines > output_height {
        total_lines.saturating_sub(output_height)
    } else {
        app.scroll_offset.min(total_lines.saturating_sub(output_height))
    };

    let visible_output: Vec<Line> = app
        .output
        .iter()
        .skip(scroll_offset)
        .take(output_height)
        .map(|line| Line::from(line.clone()))
        .collect();

    let output = Paragraph::new(visible_output)
        .style(Style::default().fg(Color::White));

    f.render_widget(output, chunks[0]);

    // Horizontal rule above input
    let rule1 = Paragraph::new("─".repeat(chunks[1].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(rule1, chunks[1]);

    // Input area with prompt
    let prompt = "rnz> ";
    let input_text = format!("{}{}", prompt, app.input);
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(input, chunks[2]);

    // Set cursor position (accounting for prompt)
    f.set_cursor_position((
        chunks[2].x + prompt.len() as u16 + app.cursor_position as u16,
        chunks[2].y,
    ));

    // Horizontal rule above status
    let rule2 = Paragraph::new("─".repeat(chunks[3].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(rule2, chunks[3]);

    // Status bar
    let status = Paragraph::new(
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" or type "),
            Span::styled("exit", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to quit | "),
            Span::styled("help", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" for commands | "),
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" history | "),
            Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" scroll"),
        ])
    )
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(status, chunks[4]);
}
