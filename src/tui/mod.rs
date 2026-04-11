use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    app::{self, AppEvent},
    engine::ExecutionError,
    provider::{AuthMethod, ProviderConfig, ProviderKind},
};

pub fn run() -> Result<(), ExecutionError> {
    let mut terminal = init_terminal()?;
    let result = run_app(&mut terminal);
    restore_terminal(&mut terminal)?;
    result
}

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

fn init_terminal() -> Result<TuiTerminal, ExecutionError> {
    enable_raw_mode().map_err(|err| ExecutionError::Io(err.to_string()))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|err| ExecutionError::Io(err.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|err| ExecutionError::Io(err.to_string()))
}

fn restore_terminal(terminal: &mut TuiTerminal) -> Result<(), ExecutionError> {
    disable_raw_mode().map_err(|err| ExecutionError::Io(err.to_string()))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|err| ExecutionError::Io(err.to_string()))?;
    terminal
        .show_cursor()
        .map_err(|err| ExecutionError::Io(err.to_string()))
}

fn run_app(terminal: &mut TuiTerminal) -> Result<(), ExecutionError> {
    let mut app = TuiApp::new()?;

    while !app.should_quit {
        app.process_worker_messages();
        terminal
            .draw(|frame| app.render(frame))
            .map_err(|err| ExecutionError::Io(err.to_string()))?;

        if event::poll(Duration::from_millis(100))
            .map_err(|err| ExecutionError::Io(err.to_string()))?
        {
            match event::read().map_err(|err| ExecutionError::Io(err.to_string()))? {
                Event::Key(key) => app.handle_key_event(key)?,
                Event::Mouse(mouse) => app.handle_mouse_event(mouse),
                _ => {}
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct FileEntry {
    relative_path: PathBuf,
    is_dir: bool,
    depth: usize,
}

#[derive(Debug)]
struct ChatEntry {
    role: &'static str,
    content: String,
}

#[derive(Debug, Clone, Copy)]
enum CommandGroup {
    Provider,
    Session,
    Model,
}

#[derive(Debug, Clone)]
enum ModalState {
    CommandPalette {
        query: String,
        selected: usize,
    },
    SessionActions {
        selected: usize,
    },
    ModelActions {
        selected: usize,
    },
    ProviderSelect {
        selected: usize,
    },
    OpenAiAuthMethod {
        selected: usize,
    },
    ProviderApiKey {
        provider: ProviderKind,
        input: String,
        has_saved_key: bool,
    },
    OpenAiCodexLogin {
        headless: bool,
        output: Vec<String>,
    },
    ProviderModels {
        provider: ProviderKind,
        models: Vec<String>,
        selected: usize,
    },
}

#[derive(Debug)]
enum WorkerMessage {
    Event(AppEvent),
    Completed(String),
    AuthProgress(String),
    ProviderModelsLoaded {
        provider: ProviderKind,
        models: Vec<String>,
    },
    Failed(String),
}

struct TuiApp {
    cwd: PathBuf,
    files: Vec<FileEntry>,
    file_index: usize,
    input: String,
    attachments: Vec<PathBuf>,
    chat: Vec<ChatEntry>,
    activity: Vec<String>,
    status: String,
    suggestions: Vec<PathBuf>,
    suggestion_index: usize,
    results_scroll: u16,
    sidebar_selected: bool,
    worker: Option<Receiver<WorkerMessage>>,
    busy: bool,
    should_quit: bool,
    frame_tick: usize,
    results_area: Rect,
    composer_area: Rect,
    modal: Option<ModalState>,
    codex_available: bool,
}

const MAX_CHAT_ENTRIES: usize = 200;
const MAX_ACTIVITY_ENTRIES: usize = 200;

impl TuiApp {
    fn new() -> Result<Self, ExecutionError> {
        let cwd = std::env::current_dir().map_err(|err| ExecutionError::Io(err.to_string()))?;
        let files = collect_entries(&cwd, &cwd)?;

        Ok(Self {
            cwd,
            files,
            file_index: 0,
            input: String::new(),
            attachments: Vec::new(),
            chat: Vec::new(),
            activity: vec!["Ready".to_string()],
            status: "Idle".to_string(),
            suggestions: Vec::new(),
            suggestion_index: 0,
            results_scroll: 0,
            sidebar_selected: false,
            worker: None,
            busy: false,
            should_quit: false,
            frame_tick: 0,
            results_area: Rect::default(),
            composer_area: Rect::default(),
            modal: None,
            codex_available: app::codex_openai_available(),
        })
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        self.frame_tick = self.frame_tick.wrapping_add(1);
        let outer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
            .split(frame.area());
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(6),
                Constraint::Min(10),
                Constraint::Length(7),
            ])
            .split(outer[1]);

        self.render_sidebar(frame, outer[0]);
        self.render_header(frame, right[0]);
        self.render_status(frame, right[1]);
        self.results_area = right[2];
        self.render_chat(frame, right[2]);
        self.composer_area = right[3];
        self.render_input(frame, right[3]);

        if self.modal.is_some() {
            self.render_modal(frame);
        }
    }

    fn render_header(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let text = Text::from(vec![
            Line::from(vec![
                Span::styled(
                    "GEOCODE",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("AI geospatial workspace", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled(
                    "System ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(
                    "Use @ for files from the current directory and / to open provider, session, and model actions.",
                ),
            ]),
        ]);

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_sidebar(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let items = self
            .files
            .iter()
            .map(|entry| {
                let marker = if entry.is_dir { "[d]" } else { "[f]" };
                let indent = "  ".repeat(entry.depth);
                ListItem::new(format!(
                    "{indent}{marker} {}",
                    entry.relative_path.display()
                ))
            })
            .collect::<Vec<_>>();

        let mut state = ListState::default().with_selected(Some(self.file_index));
        let title = format!("Workspace [{}]", self.cwd.display());
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(if self.sidebar_selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(28, 34, 48))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_status(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let spinner = spinner_frame(self.frame_tick, self.busy);
        let status_style = if self.busy {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if self.status == "Failed" {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(spinner, status_style),
            Span::raw("  "),
            Span::styled(&self.status, status_style),
        ])];

        for item in self
            .activity
            .iter()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::DarkGray)),
                Span::styled(item, Style::default().fg(Color::Gray)),
            ]));
        }

        let paragraph = Paragraph::new(Text::from(lines)).block(
            Block::default()
                .title("Live Activity")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_chat(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let lines = self.results_lines();
        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Results")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: false });
        let viewport_width = area.width.saturating_sub(2);
        let viewport_height = area.height.saturating_sub(2) as usize;
        let rendered_lines = self.approximate_results_height(viewport_width);
        let max_scroll = rendered_lines.saturating_sub(viewport_height) as u16;
        let paragraph = paragraph.scroll((self.results_scroll.min(max_scroll), 0));
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let attachments = if self.attachments.is_empty() {
            "Attachments: <none>".to_string()
        } else {
            format!(
                "Attachments: {}",
                self.attachments
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let busy = if self.busy { " [busy]" } else { "" };
        let hint = if self.sidebar_selected {
            "Sidebar focus: Enter attaches selected file, Tab returns to composer, Ctrl+C exits"
        } else {
            "Composer focus: Enter submits, PgUp/PgDn scroll results, @ opens file suggestions, / opens commands, Tab moves to sidebar"
        };
        let suggestions = if self.suggestions.is_empty() {
            "Suggestions: <none>".to_string()
        } else {
            format!(
                "Suggestions: {}",
                self.suggestions
                    .iter()
                    .enumerate()
                    .map(|(index, path)| {
                        if index == self.suggestion_index {
                            format!(">{}", path.display())
                        } else {
                            path.display().to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            )
        };
        let paragraph = Paragraph::new(format!(
            "{attachments}\n{suggestions}\n> {}\n{hint}",
            self.input
        ))
        .block(
            Block::default()
                .title(format!("Composer{busy}"))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if self.sidebar_selected {
                    Style::default().fg(Color::DarkGray)
                } else if self.busy {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Green)
                }),
        )
        .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_modal(&self, frame: &mut ratatui::Frame) {
        let area = attached_modal_area(self.composer_area, frame.area());
        frame.render_widget(Clear, area);

        match self.modal.as_ref().expect("modal checked") {
            ModalState::CommandPalette { query, selected } => {
                let items = filtered_command_groups(query)
                    .into_iter()
                    .map(|group| ListItem::new(command_group_label(group)))
                    .collect::<Vec<_>>();
                self.render_modal_list(
                    frame,
                    area,
                    "Commands",
                    &format!("Search: /{query}"),
                    items,
                    *selected,
                );
            }
            ModalState::SessionActions { selected } => {
                let items = vec![
                    ListItem::new("Show session"),
                    ListItem::new("Clear session"),
                ];
                self.render_modal_list(
                    frame,
                    area,
                    "Session Commands",
                    "Choose a session action",
                    items,
                    *selected,
                );
            }
            ModalState::ModelActions { selected } => {
                let items = vec![ListItem::new("Show configured models")];
                self.render_modal_list(
                    frame,
                    area,
                    "Model Commands",
                    "Choose a model action",
                    items,
                    *selected,
                );
            }
            ModalState::ProviderSelect { selected } => {
                let items = provider_choices()
                    .into_iter()
                    .map(|provider| ListItem::new(render_provider_choice(provider)))
                    .collect::<Vec<_>>();
                self.render_modal_list(
                    frame,
                    area,
                    "Select Provider",
                    "Choose a provider to configure",
                    items,
                    *selected,
                );
            }
            ModalState::OpenAiAuthMethod { selected } => {
                let mut items = vec![ListItem::new("API Key")];
                if self.codex_available {
                    items.push(ListItem::new("ChatGPT Plus/Pro (browser)"));
                    items.push(ListItem::new("ChatGPT Plus/Pro (headless)"));
                }
                self.render_modal_list(
                    frame,
                    area,
                    "OpenAI Auth",
                    "Choose how to authenticate OpenAI",
                    items,
                    *selected,
                );
            }
            ModalState::ProviderApiKey {
                provider,
                input,
                has_saved_key,
            } => {
                let key_value = if input.is_empty() {
                    String::new()
                } else {
                    "*".repeat(input.chars().count())
                };
                let prompt = if *has_saved_key {
                    "Enter saves a new key or keeps the stored key"
                } else {
                    "Enter saves the key and loads available models"
                };
                let text = Text::from(vec![
                    Line::from(vec![
                        Span::styled(
                            "Provider: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(provider.display_name()),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "API Key: ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(key_value),
                    ]),
                    Line::from(String::new()),
                    Line::from(prompt),
                ]);
                let paragraph = Paragraph::new(text)
                    .block(modal_block("API Key", "Esc closes"))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
            ModalState::OpenAiCodexLogin { headless, output } => {
                let prompt = if *headless {
                    "ChatGPT Plus/Pro via Codex device auth"
                } else {
                    "ChatGPT Plus/Pro via Codex browser login"
                };
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            "Provider: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("OpenAI"),
                    ]),
                    Line::from(String::new()),
                    Line::from(prompt),
                    Line::from(String::new()),
                ];
                if output.is_empty() {
                    lines.push(Line::from("Waiting for Codex login output..."));
                } else {
                    lines.extend(output.iter().map(|line| Line::from(line.clone())));
                }
                let paragraph = Paragraph::new(Text::from(lines))
                    .block(modal_block("Codex Login", "Esc hides modal"))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
            ModalState::ProviderModels {
                provider,
                models,
                selected,
            } => {
                let items = models
                    .iter()
                    .map(|model| ListItem::new(model.clone()))
                    .collect::<Vec<_>>();
                self.render_modal_list(
                    frame,
                    area,
                    "Select Model",
                    &format!("{} models", provider.display_name()),
                    items,
                    *selected,
                );
            }
        }
    }

    fn render_modal_list(
        &self,
        frame: &mut ratatui::Frame,
        area: Rect,
        title: &str,
        subtitle: &str,
        items: Vec<ListItem>,
        selected: usize,
    ) {
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(area);
        let header = Paragraph::new(subtitle).block(modal_block(title, "Esc closes"));
        frame.render_widget(header, inner[0]);

        let mut state =
            ListState::default().with_selected(Some(selected.min(items.len().saturating_sub(1))));
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(28, 34, 48))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");
        frame.render_stateful_widget(list, inner[1], &mut state);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), ExecutionError> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            self.should_quit = true;
            return Ok(());
        }

        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }

        if self.sidebar_selected {
            return self.handle_sidebar_key(key);
        }

        self.handle_composer_key(key)
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.modal.is_some() {
            return;
        }

        if !rect_contains(self.results_area, mouse.column, mouse.row) {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_results_up(3),
            MouseEventKind::ScrollDown => self.scroll_results_down(3),
            _ => {}
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) -> Result<(), ExecutionError> {
        match key.code {
            KeyCode::Tab => self.sidebar_selected = false,
            KeyCode::Up => {
                if self.file_index > 0 {
                    self.file_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.file_index + 1 < self.files.len() {
                    self.file_index += 1;
                }
            }
            KeyCode::Enter => self.attach_selected_file(),
            _ => {}
        }

        Ok(())
    }

    fn handle_composer_key(&mut self, key: KeyEvent) -> Result<(), ExecutionError> {
        match key.code {
            KeyCode::Tab => {
                if !self.suggestions.is_empty() {
                    self.accept_suggestion();
                } else {
                    self.sidebar_selected = true;
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.refresh_suggestions();
            }
            KeyCode::Esc => {
                self.suggestions.clear();
                self.suggestion_index = 0;
            }
            KeyCode::PageUp => self.scroll_results_up(8),
            KeyCode::PageDown => self.scroll_results_down(8),
            KeyCode::Home => self.results_scroll = 0,
            KeyCode::End => self.scroll_results_to_bottom(),
            KeyCode::Up => {
                if !self.suggestions.is_empty() && self.suggestion_index > 0 {
                    self.suggestion_index -= 1;
                }
            }
            KeyCode::Down => {
                if !self.suggestions.is_empty()
                    && self.suggestion_index + 1 < self.suggestions.len()
                {
                    self.suggestion_index += 1;
                }
            }
            KeyCode::Enter => {
                if !self.suggestions.is_empty() {
                    self.accept_suggestion();
                } else {
                    self.submit()?;
                }
            }
            KeyCode::Char(ch) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    if ch == '/' && self.input.is_empty() {
                        self.modal = Some(ModalState::CommandPalette {
                            query: String::new(),
                            selected: 0,
                        });
                    } else {
                        self.input.push(ch);
                        self.refresh_suggestions();
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> Result<(), ExecutionError> {
        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                return Ok(());
            }
            _ => {}
        }

        let Some(modal) = self.modal.clone() else {
            return Ok(());
        };

        match modal {
            ModalState::CommandPalette {
                mut query,
                mut selected,
            } => {
                let items = filtered_command_groups(&query);
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        if selected + 1 < items.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        selected = 0;
                    }
                    KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        query.push(ch);
                        selected = 0;
                    }
                    KeyCode::Enter => {
                        if let Some(group) = items.get(selected).copied() {
                            self.modal = Some(match group {
                                CommandGroup::Provider => {
                                    ModalState::ProviderSelect { selected: 0 }
                                }
                                CommandGroup::Session => ModalState::SessionActions { selected: 0 },
                                CommandGroup::Model => ModalState::ModelActions { selected: 0 },
                            });
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::CommandPalette { query, selected });
            }
            ModalState::SessionActions { mut selected } => {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(1),
                    KeyCode::Enter => {
                        let command = if selected == 0 {
                            "/session".to_string()
                        } else {
                            "/session clear".to_string()
                        };
                        self.execute_command_from_modal(command)?;
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::SessionActions { selected });
            }
            ModalState::ModelActions { mut selected } => {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = 0,
                    KeyCode::Enter => {
                        self.execute_command_from_modal("/model".to_string())?;
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::ModelActions { selected });
            }
            ModalState::ProviderSelect { mut selected } => {
                let max = provider_choices().len().saturating_sub(1);
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(max),
                    KeyCode::Enter => {
                        let provider = provider_choices()[selected];
                        if matches!(provider, ProviderKind::OpenAi) {
                            self.modal = Some(ModalState::OpenAiAuthMethod { selected: 0 });
                        } else if provider.requires_api_key() {
                            let has_saved_key = ProviderConfig::resolve(provider)?.configured;
                            self.modal = Some(ModalState::ProviderApiKey {
                                provider,
                                input: String::new(),
                                has_saved_key,
                            });
                        } else {
                            self.start_provider_models_worker(provider, AuthMethod::None, None)?;
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::ProviderSelect { selected });
            }
            ModalState::OpenAiAuthMethod { mut selected } => {
                let max = if self.codex_available { 2 } else { 0 };
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(max),
                    KeyCode::Enter => {
                        let provider = ProviderKind::OpenAi;
                        if selected == 0 {
                            let resolved = ProviderConfig::resolve(provider)?;
                            let has_saved_key = resolved.configured
                                && matches!(resolved.auth_method, AuthMethod::ApiKey);
                            self.modal = Some(ModalState::ProviderApiKey {
                                provider,
                                input: String::new(),
                                has_saved_key,
                            });
                        } else if selected == 1 {
                            self.start_codex_login_worker(false)?;
                        } else {
                            self.start_codex_login_worker(true)?;
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::OpenAiAuthMethod { selected });
            }
            ModalState::ProviderApiKey {
                provider,
                mut input,
                has_saved_key,
            } => {
                match key.code {
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        input.push(ch);
                    }
                    KeyCode::Enter => {
                        if input.trim().is_empty() && !has_saved_key {
                            self.status = "API key required".to_string();
                            self.push_activity(format!(
                                "{} needs an API key",
                                provider.display_name()
                            ));
                        } else {
                            let next_key = if input.trim().is_empty() {
                                None
                            } else {
                                Some(input.trim().to_string())
                            };
                            self.start_provider_models_worker(
                                provider,
                                AuthMethod::ApiKey,
                                next_key,
                            )?;
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::ProviderApiKey {
                    provider,
                    input,
                    has_saved_key,
                });
            }
            ModalState::OpenAiCodexLogin { headless, output } => {
                self.modal = Some(ModalState::OpenAiCodexLogin { headless, output });
            }
            ModalState::ProviderModels {
                provider,
                models,
                mut selected,
            } => {
                let max = models.len().saturating_sub(1);
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(max),
                    KeyCode::Enter => {
                        if let Some(model) = models.get(selected).cloned() {
                            self.start_provider_setup_worker(provider, model)?;
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                self.modal = Some(ModalState::ProviderModels {
                    provider,
                    models,
                    selected,
                });
            }
        }

        Ok(())
    }

    fn attach_selected_file(&mut self) {
        let Some(entry) = self.files.get(self.file_index) else {
            return;
        };
        if entry.is_dir {
            return;
        }
        self.attach_file(entry.relative_path.clone());
    }

    fn attach_file(&mut self, path: PathBuf) {
        if self.attachments.iter().all(|existing| existing != &path) {
            self.attachments.push(path.clone());
        }
        self.push_activity(format!("Attached {}", path.display()));
        self.status = format!("Attached {}", path.display());
    }

    fn refresh_suggestions(&mut self) {
        let Some(query) = active_attachment_query(&self.input) else {
            self.suggestions.clear();
            self.suggestion_index = 0;
            return;
        };

        self.suggestions = self
            .files
            .iter()
            .filter(|entry| !entry.is_dir)
            .filter(|entry| entry.relative_path.display().to_string().contains(&query))
            .map(|entry| entry.relative_path.clone())
            .take(10)
            .collect();
        self.suggestion_index = 0;
    }

    fn accept_suggestion(&mut self) {
        let Some(path) = self.suggestions.get(self.suggestion_index).cloned() else {
            return;
        };
        replace_active_attachment_query(&mut self.input, &path);
        self.attach_file(path);
        self.suggestions.clear();
        self.suggestion_index = 0;
    }

    fn submit(&mut self) -> Result<(), ExecutionError> {
        if self.busy || self.input.trim().is_empty() {
            return Ok(());
        }

        let input = std::mem::take(&mut self.input);
        let attachments = self.attachments.clone();
        let prompt = if attachments.is_empty() {
            input.clone()
        } else {
            format!(
                "{}\nFiles: {}",
                input,
                attachments
                    .iter()
                    .map(|path| format!("@{}", path.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        self.suggestions.clear();
        self.suggestion_index = 0;
        self.attachments.clear();
        self.start_worker("user", prompt, input, attachments)
    }

    fn process_worker_messages(&mut self) {
        let mut disconnected = false;
        let mut pending = Vec::new();

        if let Some(receiver) = &self.worker {
            loop {
                match receiver.try_recv() {
                    Ok(message) => pending.push(message),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        for message in pending {
            match message {
                WorkerMessage::Event(event) => self.handle_app_event(event),
                WorkerMessage::Completed(text) => {
                    self.push_chat(ChatEntry {
                        role: "assistant",
                        content: text,
                    });
                    self.scroll_results_to_bottom();
                    self.status = "Completed".to_string();
                    self.push_activity("Result ready".to_string());
                    self.busy = false;
                }
                WorkerMessage::AuthProgress(line) => {
                    if let Some(ModalState::OpenAiCodexLogin { headless, output }) =
                        self.modal.take()
                    {
                        let mut output = output;
                        output.push(line.clone());
                        if output.len() > 24 {
                            let overflow = output.len() - 24;
                            output.drain(0..overflow);
                        }
                        self.modal = Some(ModalState::OpenAiCodexLogin { headless, output });
                    }
                    self.status = line;
                }
                WorkerMessage::ProviderModelsLoaded { provider, models } => {
                    let current_model = ProviderConfig::resolve(provider)
                        .map(|config| config.model)
                        .unwrap_or_default();
                    let selected = models
                        .iter()
                        .position(|model| model == &current_model)
                        .unwrap_or(0);
                    self.modal = Some(ModalState::ProviderModels {
                        provider,
                        models,
                        selected,
                    });
                    self.status = format!("{} models loaded", provider.display_name());
                    self.push_activity(self.status.clone());
                    self.busy = false;
                }
                WorkerMessage::Failed(error) => {
                    self.modal = None;
                    self.push_chat(ChatEntry {
                        role: "error",
                        content: error.clone(),
                    });
                    self.scroll_results_to_bottom();
                    self.push_activity(format!("Error: {error}"));
                    self.status = "Failed".to_string();
                    self.busy = false;
                }
            }
        }

        if disconnected || !self.busy {
            self.worker = None;
        }
    }

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Message(message) => {
                self.status = message.clone();
                self.push_activity(message);
            }
            AppEvent::PlanningStarted => {
                self.status = "Planning".to_string();
                self.push_activity("Planning started".to_string());
            }
            AppEvent::PlanningFinished { millis } => {
                self.status = format!("Planning finished in {millis} ms");
                self.push_activity(self.status.clone());
            }
            AppEvent::ExecutionStarted { steps } => {
                self.status = format!("Executing {steps} steps");
                self.push_activity(self.status.clone());
            }
            AppEvent::ExecutionStepCompleted {
                step_id,
                capability,
            } => {
                self.status = format!("Completed {step_id} ({capability})");
                self.push_activity(self.status.clone());
            }
            AppEvent::ExecutionFinished { millis } => {
                self.status = format!("Execution finished in {millis} ms");
                self.push_activity(self.status.clone());
            }
        }
    }

    fn results_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for entry in &self.chat {
            lines.extend(render_chat_entry(entry));
            lines.push(Line::from(String::new()));
        }

        if lines.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No results yet",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        lines
    }

    fn approximate_results_height(&self, width: u16) -> usize {
        let width = width.max(1) as usize;

        if self.chat.is_empty() {
            return 1;
        }

        self.chat
            .iter()
            .map(|entry| {
                let body_height = entry
                    .content
                    .lines()
                    .map(|line| {
                        let line_width = line.chars().count().max(1);
                        line_width.div_ceil(width)
                    })
                    .sum::<usize>();

                1 + body_height + 1
            })
            .sum()
    }

    fn scroll_results_up(&mut self, amount: u16) {
        self.results_scroll = self.results_scroll.saturating_sub(amount);
    }

    fn scroll_results_down(&mut self, amount: u16) {
        self.results_scroll = self.results_scroll.saturating_add(amount);
    }

    fn scroll_results_to_bottom(&mut self) {
        let line_count = self.results_lines().len();
        self.results_scroll = line_count.saturating_sub(1).min(u16::MAX as usize) as u16;
    }

    fn execute_command_from_modal(&mut self, command: String) -> Result<(), ExecutionError> {
        self.modal = None;
        self.start_worker("user", command.clone(), command, Vec::new())
    }

    fn push_chat(&mut self, entry: ChatEntry) {
        self.chat.push(entry);
        if self.chat.len() > MAX_CHAT_ENTRIES {
            let overflow = self.chat.len() - MAX_CHAT_ENTRIES;
            self.chat.drain(0..overflow);
        }
    }

    fn push_activity(&mut self, entry: String) {
        self.activity.push(entry);
        if self.activity.len() > MAX_ACTIVITY_ENTRIES {
            let overflow = self.activity.len() - MAX_ACTIVITY_ENTRIES;
            self.activity.drain(0..overflow);
        }
    }

    fn start_provider_models_worker(
        &mut self,
        provider: ProviderKind,
        auth_method: AuthMethod,
        credential: Option<String>,
    ) -> Result<(), ExecutionError> {
        if self.busy {
            return Ok(());
        }

        self.status = format!("Loading {} models", provider.display_name());
        self.push_activity(self.status.clone());
        self.busy = true;

        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<String>, ExecutionError> {
                app::set_provider_auth_method(provider, auth_method)?;
                if let Some(credential) = credential {
                    if matches!(auth_method, AuthMethod::OAuth) {
                        app::store_provider_oauth_token(provider, credential)?;
                    } else if matches!(auth_method, AuthMethod::ApiKey) {
                        app::store_provider_api_key(provider, credential)?;
                    }
                }
                app::fetch_provider_models(provider)
            })();

            match result {
                Ok(models) => {
                    let _ = tx.send(WorkerMessage::ProviderModelsLoaded { provider, models });
                }
                Err(error) => {
                    let _ = tx.send(WorkerMessage::Failed(error.to_string()));
                }
            }
        });

        Ok(())
    }

    fn start_codex_login_worker(&mut self, headless: bool) -> Result<(), ExecutionError> {
        if self.busy {
            return Ok(());
        }

        self.status = if headless {
            "Waiting for Codex headless login".to_string()
        } else {
            "Waiting for Codex browser login".to_string()
        };
        self.push_activity(self.status.clone());
        self.busy = true;
        self.modal = Some(ModalState::OpenAiCodexLogin {
            headless,
            output: Vec::new(),
        });

        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<String>, ExecutionError> {
                app::login_provider_via_codex(
                    ProviderKind::OpenAi,
                    if headless {
                        crate::auth::CodexLoginMode::Headless
                    } else {
                        crate::auth::CodexLoginMode::Browser
                    },
                    |line| {
                        let _ = tx.send(WorkerMessage::AuthProgress(line));
                    },
                )?;
                app::fetch_provider_models(ProviderKind::OpenAi)
            })();

            match result {
                Ok(models) => {
                    let _ = tx.send(WorkerMessage::ProviderModelsLoaded {
                        provider: ProviderKind::OpenAi,
                        models,
                    });
                }
                Err(error) => {
                    let _ = tx.send(WorkerMessage::Failed(error.to_string()));
                }
            }
        });

        Ok(())
    }

    fn start_provider_setup_worker(
        &mut self,
        provider: ProviderKind,
        model: String,
    ) -> Result<(), ExecutionError> {
        if self.busy {
            return Ok(());
        }

        self.status = format!("Saving {} configuration", provider.display_name());
        self.push_activity(self.status.clone());
        self.busy = true;

        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let result = app::setup_provider(provider, None, model);

            match result {
                Ok(text) => {
                    let _ = tx.send(WorkerMessage::Completed(text));
                }
                Err(error) => {
                    let _ = tx.send(WorkerMessage::Failed(error.to_string()));
                }
            }
        });

        self.modal = None;
        Ok(())
    }

    fn start_worker(
        &mut self,
        role: &'static str,
        transcript: String,
        input: String,
        attachments: Vec<PathBuf>,
    ) -> Result<(), ExecutionError> {
        self.push_chat(ChatEntry {
            role,
            content: transcript,
        });
        self.scroll_results_to_bottom();
        self.status = "Running request".to_string();
        self.push_activity("Request submitted".to_string());
        self.busy = true;

        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let result = app::execute_tui_input(input, attachments, |event| {
                let _ = tx.send(WorkerMessage::Event(event));
            });

            match result {
                Ok(text) => {
                    let _ = tx.send(WorkerMessage::Completed(text));
                }
                Err(error) => {
                    let _ = tx.send(WorkerMessage::Failed(error.to_string()));
                }
            }
        });

        Ok(())
    }
}

fn spinner_frame(frame_tick: usize, busy: bool) -> &'static str {
    if !busy {
        return "●";
    }

    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[frame_tick % FRAMES.len()]
}

fn render_chat_entry(entry: &ChatEntry) -> Vec<Line<'static>> {
    let (label, accent, body) = match entry.role {
        "user" => ("You", Color::Cyan, Style::default().fg(Color::White)),
        "assistant" => ("Result", Color::Green, Style::default().fg(Color::White)),
        "error" => ("Error", Color::Red, Style::default().fg(Color::White)),
        _ => ("Note", Color::Yellow, Style::default().fg(Color::White)),
    };

    let mut lines = vec![Line::from(vec![
        Span::styled(
            label,
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ─────────────────", Style::default().fg(Color::DarkGray)),
    ])];

    for raw_line in entry.content.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            lines.push(Line::from(String::new()));
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(": ") {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{key}: "),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(value.to_string(), body),
            ]));
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(accent)),
                Span::styled(item.to_string(), body),
            ]));
            continue;
        }

        lines.push(Line::from(vec![Span::styled(trimmed.to_string(), body)]));
    }

    lines
}

fn collect_entries(root: &Path, current: &Path) -> Result<Vec<FileEntry>, ExecutionError> {
    let mut entries = fs::read_dir(current)
        .map_err(|err| ExecutionError::Io(err.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| ExecutionError::Io(err.to_string()))?;
    entries.sort_by(|left, right| {
        let left_is_dir = left.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let right_is_dir = right.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        right_is_dir
            .cmp(&left_is_dir)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });

    let mut visible = Vec::new();
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        let is_dir = entry
            .file_type()
            .map_err(|err| ExecutionError::Io(err.to_string()))?
            .is_dir();
        let depth = relative_path.components().count().saturating_sub(1);
        visible.push(FileEntry {
            relative_path: relative_path.clone(),
            is_dir,
            depth,
        });

        if is_dir {
            visible.extend(collect_entries(root, &path)?);
        }
    }

    Ok(visible)
}

fn active_attachment_query(input: &str) -> Option<String> {
    let token = input.split_whitespace().last()?;
    token.strip_prefix('@').map(ToString::to_string)
}

fn replace_active_attachment_query(input: &mut String, path: &Path) {
    let replacement = format!("@{} ", path.display());
    let split_index = input
        .rfind(char::is_whitespace)
        .map(|index| index + 1)
        .unwrap_or(0);
    input.replace_range(split_index.., &replacement);
}

fn attached_modal_area(composer_area: Rect, frame_area: Rect) -> Rect {
    let width = composer_area.width.min(54).max(36);
    let height: u16 = 10;
    let x = composer_area
        .x
        .saturating_add(composer_area.width.saturating_sub(width) / 2);
    let preferred_y = composer_area.y.saturating_sub(height.saturating_sub(1));
    let y = preferred_y.max(frame_area.y);

    Rect {
        x,
        y,
        width,
        height: height.min(frame_area.height),
    }
}

fn modal_block<'a>(title: &'a str, help: &'a str) -> Block<'a> {
    Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black))
}

fn provider_choices() -> [ProviderKind; 3] {
    ProviderKind::all()
}

fn render_provider_choice(provider: ProviderKind) -> String {
    let status = ProviderConfig::resolve(provider).ok();
    let configured = status.as_ref().is_some_and(|config| config.configured);
    let marker = if configured {
        "configured"
    } else {
        "setup needed"
    };
    format!("{} ({marker})", provider.display_name())
}

fn filtered_command_groups(query: &str) -> Vec<CommandGroup> {
    let normalized = query.trim().to_ascii_lowercase();
    [
        CommandGroup::Provider,
        CommandGroup::Session,
        CommandGroup::Model,
    ]
    .into_iter()
    .filter(|group| command_group_label(*group).contains(&normalized))
    .collect()
}

fn command_group_label(group: CommandGroup) -> &'static str {
    match group {
        CommandGroup::Provider => "provider",
        CommandGroup::Session => "session",
        CommandGroup::Model => "model",
    }
}

fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::{active_attachment_query, replace_active_attachment_query};
    use std::path::Path;

    #[test]
    fn detects_attachment_query_from_current_token() {
        assert_eq!(
            active_attachment_query("compare @src/ma"),
            Some("src/ma".into())
        );
        assert_eq!(active_attachment_query("compare files"), None);
    }

    #[test]
    fn replaces_current_attachment_token() {
        let mut input = "compare @src/ma".to_string();
        replace_active_attachment_query(&mut input, Path::new("src/main.rs"));
        assert_eq!(input, "compare @src/main.rs ");
    }
}
