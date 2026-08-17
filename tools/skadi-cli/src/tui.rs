use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};
use std::{fs, io, path::Path, time::Duration};

use crate::actions::{
    self, ActionError, AnalysisFact, AnalysisFactLevel, AnalysisSubjectKind, BuildOptions,
    BuildResult, CheckResult, DebugOptions, DebugPrepared, DiagnosticSummary, DoctorReport,
    FailureSource, FormatOptions, FormatState, ManifestConfigResult, ProjectSummary, RunResult,
};
use crate::debug_session::{
    DebugCommand, DebugEvent, DebugIoMode, DebugSession, DebugStop, start_debug_session,
};

const MIN_WIDTH: u16 = 96;
const MIN_HEIGHT: u16 = 28;

pub fn run(args: &[String]) -> Result<(), String> {
    if matches!(args.first().map(|s| s.as_str()), Some("--help" | "-h")) {
        print_help();
        return Ok(());
    }
    if matches!(args.first().map(|s| s.as_str()), Some("--smoke-test")) {
        let app = App::new();
        if !app.project.summary.loaded {
            println!("tui smoke ok (no project)");
        } else {
            println!("tui smoke ok");
        }
        return Ok(());
    }

    let mut stdout = io::stdout();
    enable_raw_mode().map_err(|e| format!("failed to enable raw mode: {e}"))?;
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("failed to enter alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("failed to create terminal: {e}"))?;
    terminal
        .clear()
        .map_err(|e| format!("failed to clear terminal: {e}"))?;

    let mut app = App::new();
    let run_result = run_app(&mut terminal, &mut app);

    let restore_result = restore_terminal(&mut terminal);
    match (run_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(run_err), Err(restore_err)) => {
            Err(format!("{run_err}\nterminal restore error: {restore_err}"))
        }
    }
}

fn print_help() {
    println!("skadi tui");
    println!(
        "Full-screen interactive workflow for Skadi v{}.",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Keys:");
    println!("  q              Quit");
    println!("  Tab            Next screen");
    println!("  Shift+Tab      Previous screen");
    println!("  c/b/r/f/d      Check / Build / Run / Format / Doctor");
    println!("  p/m/e/l/x/h    Home / Config / Diagnostics / Lifecycle / Debug / Help");
    println!("  F5/F10/F8      Start or continue / Step / Stop debug session");
    println!("  o/n/i          Open project / New project / Init directory (Bootstrap view)");
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), String> {
    disable_raw_mode().map_err(|e| format!("failed to disable raw mode: {e}"))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("failed to leave alternate screen: {e}"))?;
    terminal
        .show_cursor()
        .map_err(|e| format!("failed to show cursor: {e}"))?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        app.poll_debug_events();
        terminal
            .draw(|frame| render(frame, app))
            .map_err(|e| format!("failed to draw TUI: {e}"))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(200)).map_err(|e| format!("event poll failed: {e}"))? {
            let Event::Key(key) = event::read().map_err(|e| format!("event read failed: {e}"))?
            else {
                continue;
            };
            if key.kind == KeyEventKind::Press {
                app.handle_key(key)?;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppScreen {
    Home,
    Config,
    Diagnostics,
    Lifecycle,
    Debug,
    BuildRun,
    Doctor,
    Bootstrap,
    Help,
}

impl AppScreen {
    fn title(self) -> &'static str {
        match self {
            Self::Home => "Project",
            Self::Config => "Config",
            Self::Diagnostics => "Diagnostics",
            Self::Lifecycle => "Lifecycle",
            Self::Debug => "Debug",
            Self::BuildRun => "Build/Run",
            Self::Doctor => "Doctor",
            Self::Bootstrap => "Bootstrap",
            Self::Help => "Help",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Home => Self::Config,
            Self::Config => Self::Diagnostics,
            Self::Diagnostics => Self::Lifecycle,
            Self::Lifecycle => Self::Debug,
            Self::Debug => Self::BuildRun,
            Self::BuildRun => Self::Doctor,
            Self::Doctor => Self::Bootstrap,
            Self::Bootstrap => Self::Help,
            Self::Help => Self::Home,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Home => Self::Help,
            Self::Config => Self::Home,
            Self::Diagnostics => Self::Config,
            Self::Lifecycle => Self::Diagnostics,
            Self::Debug => Self::Lifecycle,
            Self::BuildRun => Self::Debug,
            Self::Doctor => Self::BuildRun,
            Self::Bootstrap => Self::Doctor,
            Self::Help => Self::Bootstrap,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppFocus {
    Tabs,
    ConfigFields,
    ConfigInput,
    DiagnosticsHistory,
    DiagnosticsList,
    LifecycleSubjects,
    LifecycleFacts,
    DebugFrames,
    BootstrapInput,
}

#[derive(Clone, Debug)]
struct ActionState {
    name: String,
    ok: bool,
    source: Option<FailureSource>,
    summary: String,
    detail: Vec<String>,
}

#[derive(Clone, Debug)]
struct ProjectState {
    summary: ProjectSummary,
    manifest_preview: Vec<String>,
    build_artifacts: Vec<String>,
    manifest_exists: bool,
    entry_exists: bool,
    build_dir_exists: bool,
}

#[derive(Clone, Debug)]
struct DiagnosticsRecord {
    action: String,
    ok: bool,
    source: Option<FailureSource>,
    summary: String,
    diagnostics: Vec<DiagnosticSummary>,
    analysis: Vec<AnalysisFact>,
    detail: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct DiagnosticsState {
    history: Vec<DiagnosticsRecord>,
    history_selected: usize,
    selected: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LifecycleFilter {
    #[default]
    All,
    Tasks,
    Channels,
    Memory,
    Resources,
}

impl LifecycleFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Tasks => "Tasks",
            Self::Channels => "Channels",
            Self::Memory => "Memory",
            Self::Resources => "Resources",
        }
    }

    fn accepts(self, kind: AnalysisSubjectKind) -> bool {
        match self {
            Self::All => true,
            Self::Tasks => kind == AnalysisSubjectKind::Task,
            Self::Channels => kind == AnalysisSubjectKind::Channel,
            Self::Memory => kind == AnalysisSubjectKind::Memory,
            Self::Resources => kind == AnalysisSubjectKind::Resource,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct LifecycleState {
    filter: LifecycleFilter,
    selected_subject: usize,
    selected_fact: usize,
}

#[derive(Clone, Debug)]
struct LifecycleSubject<'a> {
    id: &'a str,
    name: &'a str,
    kind: AnalysisSubjectKind,
    facts: Vec<&'a AnalysisFact>,
}

#[derive(Clone, Debug, Default)]
struct EnvironmentState {
    report: Option<DoctorReport>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DebugRunState {
    #[default]
    Idle,
    Starting,
    Running,
    Stopped,
    Exited,
    Failed,
}

impl DebugRunState {
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Exited => "exited",
            Self::Failed => "failed",
        }
    }
}

#[derive(Default)]
struct DebugState {
    session: Option<DebugSession>,
    prepared: Option<DebugPrepared>,
    run_state: DebugRunState,
    current_stop: Option<DebugStop>,
    selected_frame: usize,
    output: Vec<String>,
    exit_status: Option<String>,
}

#[derive(Clone, Debug)]
struct StatusLine {
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigField {
    Name,
    Version,
    Edition,
    Entry,
    Target,
    Compiler,
}

impl ConfigField {
    fn all() -> [Self; 6] {
        [
            Self::Name,
            Self::Version,
            Self::Edition,
            Self::Entry,
            Self::Target,
            Self::Compiler,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Version => "version",
            Self::Edition => "edition",
            Self::Entry => "entry",
            Self::Target => "build target",
            Self::Compiler => "preferred compiler",
        }
    }

    fn is_manifest_field(self) -> bool {
        matches!(
            self,
            Self::Name | Self::Version | Self::Edition | Self::Entry
        )
    }
}

#[derive(Clone, Debug, Default)]
struct ConfigState {
    manifest: Option<ManifestConfigResult>,
    saved_manifest: Option<ManifestConfigResult>,
    selected: usize,
    editing: bool,
    edit_buffer: String,
}

#[derive(Clone, Debug)]
struct BuildPrefs {
    target: String,
    compiler: Option<String>,
}

impl Default for BuildPrefs {
    fn default() -> Self {
        Self {
            target: "host".to_string(),
            compiler: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BootstrapMode {
    Idle,
    NewProject,
    OpenProject,
}

#[derive(Clone, Debug)]
struct BootstrapState {
    mode: BootstrapMode,
    input: String,
}

impl Default for BootstrapState {
    fn default() -> Self {
        Self {
            mode: BootstrapMode::Idle,
            input: String::new(),
        }
    }
}

struct App {
    screen: AppScreen,
    focus: AppFocus,
    should_quit: bool,
    project: ProjectState,
    config: ConfigState,
    build_prefs: BuildPrefs,
    diagnostics: DiagnosticsState,
    lifecycle: LifecycleState,
    environment: EnvironmentState,
    debug: DebugState,
    status: StatusLine,
    last_action: Option<ActionState>,
    last_build: Option<BuildResult>,
    last_run: Option<RunResult>,
    bootstrap: BootstrapState,
}

impl App {
    fn new() -> Self {
        let project = load_project_state(&actions::project_summary());
        let mut app = Self {
            screen: AppScreen::Home,
            focus: AppFocus::Tabs,
            should_quit: false,
            project,
            config: ConfigState::default(),
            build_prefs: BuildPrefs::default(),
            diagnostics: DiagnosticsState::default(),
            lifecycle: LifecycleState::default(),
            environment: EnvironmentState::default(),
            debug: DebugState::default(),
            status: StatusLine {
                text: "Ready. Press 'h' for keys.".to_string(),
            },
            last_action: None,
            last_build: None,
            last_run: None,
            bootstrap: BootstrapState::default(),
        };
        app.refresh_config();
        app
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), String> {
        if self.screen == AppScreen::Debug {
            match key.code {
                KeyCode::F(5) => {
                    self.debug_start_or_continue();
                    return Ok(());
                }
                KeyCode::F(10) => {
                    self.debug_command(DebugCommand::Step);
                    return Ok(());
                }
                KeyCode::F(8) => {
                    self.debug_command(DebugCommand::Quit);
                    return Ok(());
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('q') => {
                if let Some(session) = &self.debug.session {
                    let _ = session.terminate();
                }
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Tab => {
                self.screen = self.screen.next();
                self.focus = AppFocus::Tabs;
                return Ok(());
            }
            KeyCode::BackTab => {
                self.screen = self.screen.previous();
                self.focus = AppFocus::Tabs;
                return Ok(());
            }
            _ => {}
        }

        match key.code {
            KeyCode::Char('c') => self.run_check(),
            KeyCode::Char('b') => self.run_build(),
            KeyCode::Char('r') => self.run_run(),
            KeyCode::Char('f') => self.run_format(),
            KeyCode::Char('d') => self.run_doctor(),
            KeyCode::Char('p') => {
                self.screen = AppScreen::Home;
                self.focus = AppFocus::Tabs;
                self.refresh_project();
                Ok(())
            }
            KeyCode::Char('m') => {
                self.screen = AppScreen::Config;
                self.focus = AppFocus::ConfigFields;
                self.refresh_config();
                Ok(())
            }
            KeyCode::Char('e') => {
                self.screen = AppScreen::Diagnostics;
                self.focus = if self.diagnostics.history.is_empty() {
                    AppFocus::DiagnosticsList
                } else {
                    AppFocus::DiagnosticsHistory
                };
                Ok(())
            }
            KeyCode::Char('l') => {
                self.screen = AppScreen::Lifecycle;
                self.focus = AppFocus::LifecycleSubjects;
                self.clamp_lifecycle_selection();
                Ok(())
            }
            KeyCode::Char('x') => {
                self.screen = AppScreen::Debug;
                self.focus = AppFocus::DebugFrames;
                Ok(())
            }
            KeyCode::Char('h') => {
                self.screen = AppScreen::Help;
                self.focus = AppFocus::Tabs;
                Ok(())
            }
            KeyCode::Char('o') => {
                self.screen = AppScreen::Bootstrap;
                self.bootstrap.mode = BootstrapMode::OpenProject;
                self.bootstrap.input.clear();
                self.focus = AppFocus::BootstrapInput;
                self.status.text = "Enter a project directory path and press Enter.".to_string();
                Ok(())
            }
            _ => self.handle_screen_specific_key(key),
        }
    }

    fn handle_screen_specific_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match self.screen {
            AppScreen::Config => self.handle_config_key(key),
            AppScreen::Diagnostics => self.handle_diagnostics_key(key),
            AppScreen::Lifecycle => self.handle_lifecycle_key(key),
            AppScreen::Debug => self.handle_debug_key(key),
            AppScreen::Bootstrap => self.handle_bootstrap_key(key),
            _ => Ok(()),
        }
    }

    fn handle_config_key(&mut self, key: KeyEvent) -> Result<(), String> {
        if self.config.editing {
            match key.code {
                KeyCode::Esc => {
                    self.config.editing = false;
                    self.config.edit_buffer.clear();
                    self.focus = AppFocus::ConfigFields;
                    self.status.text = "Cancelled config edit.".to_string();
                }
                KeyCode::Backspace => {
                    self.config.edit_buffer.pop();
                }
                KeyCode::Enter => {
                    self.commit_config_edit();
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.config.edit_buffer.push(ch);
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.config.selected = (self.config.selected + 1).min(ConfigField::all().len() - 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.config.selected = self.config.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                self.start_config_edit();
            }
            KeyCode::Char('s') => {
                self.save_config();
            }
            KeyCode::Char('g') => {
                self.generate_entry_file();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_diagnostics_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Left => {
                self.focus = AppFocus::DiagnosticsHistory;
            }
            KeyCode::Right => {
                self.focus = AppFocus::DiagnosticsList;
            }
            KeyCode::Down | KeyCode::Char('j') => match self.focus {
                AppFocus::DiagnosticsHistory if !self.diagnostics.history.is_empty() => {
                    self.diagnostics.history_selected = (self.diagnostics.history_selected + 1)
                        .min(self.diagnostics.history.len() - 1);
                    self.diagnostics.selected = 0;
                }
                AppFocus::DiagnosticsHistory => {}
                AppFocus::DiagnosticsList => {
                    let item_count = self.current_diagnostic_items_len();
                    if item_count > 0 {
                        self.diagnostics.selected =
                            (self.diagnostics.selected + 1).min(item_count - 1);
                    }
                }
                _ => {}
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focus {
                AppFocus::DiagnosticsHistory => {
                    self.diagnostics.history_selected =
                        self.diagnostics.history_selected.saturating_sub(1);
                    self.diagnostics.selected = 0;
                }
                AppFocus::DiagnosticsList => {
                    self.diagnostics.selected = self.diagnostics.selected.saturating_sub(1);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_lifecycle_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Left => self.focus = AppFocus::LifecycleSubjects,
            KeyCode::Right => self.focus = AppFocus::LifecycleFacts,
            KeyCode::Char('1') => self.set_lifecycle_filter(LifecycleFilter::All),
            KeyCode::Char('2') => self.set_lifecycle_filter(LifecycleFilter::Tasks),
            KeyCode::Char('3') => self.set_lifecycle_filter(LifecycleFilter::Channels),
            KeyCode::Char('4') => self.set_lifecycle_filter(LifecycleFilter::Memory),
            KeyCode::Char('5') => self.set_lifecycle_filter(LifecycleFilter::Resources),
            KeyCode::Down | KeyCode::Char('j') => match self.focus {
                AppFocus::LifecycleSubjects => {
                    let count = self.current_lifecycle_subjects().len();
                    if count > 0 {
                        self.lifecycle.selected_subject =
                            (self.lifecycle.selected_subject + 1).min(count - 1);
                        self.lifecycle.selected_fact = 0;
                    }
                }
                AppFocus::LifecycleFacts => {
                    let count = self.current_lifecycle_fact_count();
                    if count > 0 {
                        self.lifecycle.selected_fact =
                            (self.lifecycle.selected_fact + 1).min(count - 1);
                    }
                }
                _ => {}
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focus {
                AppFocus::LifecycleSubjects => {
                    self.lifecycle.selected_subject =
                        self.lifecycle.selected_subject.saturating_sub(1);
                    self.lifecycle.selected_fact = 0;
                }
                AppFocus::LifecycleFacts => {
                    self.lifecycle.selected_fact = self.lifecycle.selected_fact.saturating_sub(1);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_debug_key(&mut self, key: KeyEvent) -> Result<(), String> {
        let frame_count = self
            .debug
            .current_stop
            .as_ref()
            .map(|stop| stop.frames.len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') if frame_count > 0 => {
                self.debug.selected_frame =
                    (self.debug.selected_frame + 1).min(frame_count.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') if frame_count > 0 => {
                self.debug.selected_frame = self.debug.selected_frame.saturating_sub(1);
            }
            KeyCode::Enter => self.debug_start_or_continue(),
            _ => {}
        }
        Ok(())
    }

    fn set_lifecycle_filter(&mut self, filter: LifecycleFilter) {
        self.lifecycle.filter = filter;
        self.lifecycle.selected_subject = 0;
        self.lifecycle.selected_fact = 0;
        self.focus = AppFocus::LifecycleSubjects;
    }

    fn handle_bootstrap_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match self.bootstrap.mode {
            BootstrapMode::Idle => match key.code {
                KeyCode::Char('o') => {
                    self.bootstrap.mode = BootstrapMode::OpenProject;
                    self.bootstrap.input.clear();
                    self.focus = AppFocus::BootstrapInput;
                    self.status.text =
                        "Enter a project directory path and press Enter.".to_string();
                }
                KeyCode::Char('n') => {
                    self.bootstrap.mode = BootstrapMode::NewProject;
                    self.bootstrap.input.clear();
                    self.focus = AppFocus::BootstrapInput;
                    self.status.text = "Enter a project name and press Enter.".to_string();
                }
                KeyCode::Char('i') => {
                    let result = actions::init_project_at(&self.project.summary.cwd);
                    match result {
                        Ok(ok) => {
                            self.set_project_summary(actions::project_summary_at(&ok.root));
                            self.status.text =
                                format!("Initialized project in {}", ok.root.display());
                            self.last_action = Some(ActionState {
                                name: "init".to_string(),
                                ok: true,
                                source: None,
                                summary: format!(
                                    "Initialized directory{}",
                                    ok.name
                                        .as_ref()
                                        .map(|name| format!(" for '{name}'"))
                                        .unwrap_or_default()
                                ),
                                detail: vec![format!("root: {}", ok.root.display())],
                            });
                        }
                        Err(err) => self.apply_error("init", err),
                    }
                }
                _ => {}
            },
            BootstrapMode::NewProject => match key.code {
                KeyCode::Esc => {
                    self.bootstrap.mode = BootstrapMode::Idle;
                    self.bootstrap.input.clear();
                    self.focus = AppFocus::Tabs;
                    self.status.text = "Cancelled project creation.".to_string();
                }
                KeyCode::Backspace => {
                    self.bootstrap.input.pop();
                }
                KeyCode::Enter => {
                    let name = self.bootstrap.input.trim().to_string();
                    if name.is_empty() {
                        self.status.text = "Project name cannot be empty.".to_string();
                    } else {
                        match actions::create_new_project_at(&self.project.summary.cwd, &name) {
                            Ok(ok) => {
                                self.bootstrap.mode = BootstrapMode::Idle;
                                self.bootstrap.input.clear();
                                self.focus = AppFocus::Tabs;
                                self.set_project_summary(actions::project_summary_at(&ok.root));
                                self.last_action = Some(ActionState {
                                    name: "new".to_string(),
                                    ok: true,
                                    source: None,
                                    summary: format!(
                                        "Created project '{}'",
                                        ok.name.as_deref().unwrap_or(&name)
                                    ),
                                    detail: vec![format!("root: {}", ok.root.display())],
                                });
                                self.status.text =
                                    format!("Created and opened project '{}'.", name);
                                self.screen = AppScreen::Home;
                            }
                            Err(err) => self.apply_error("new", err),
                        }
                    }
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.bootstrap.input.push(ch);
                }
                _ => {}
            },
            BootstrapMode::OpenProject => match key.code {
                KeyCode::Esc => {
                    self.bootstrap.mode = BootstrapMode::Idle;
                    self.bootstrap.input.clear();
                    self.focus = AppFocus::Tabs;
                    self.status.text = "Cancelled project switch.".to_string();
                }
                KeyCode::Backspace => {
                    self.bootstrap.input.pop();
                }
                KeyCode::Enter => {
                    let input = self.bootstrap.input.trim().to_string();
                    if input.is_empty() {
                        self.status.text = "Project path cannot be empty.".to_string();
                    } else {
                        let path = std::path::PathBuf::from(&input);
                        let root = if path.is_absolute() {
                            path
                        } else {
                            self.project.summary.cwd.join(path)
                        };
                        self.open_project_path(root);
                    }
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.bootstrap.input.push(ch);
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn run_check(&mut self) -> Result<(), String> {
        match actions::run_check_at(&self.project.summary.cwd) {
            Ok(result) => {
                self.apply_check_result(&result);
                self.set_project_summary(result.project.clone());
                self.status.text = format!("check ok: {}", result.entry.display());
                self.last_action = Some(ActionState {
                    name: "check".to_string(),
                    ok: true,
                    source: None,
                    summary: format!("Frontend check passed for {}", result.entry.display()),
                    detail: result.warnings.iter().map(format_diagnostic_line).collect(),
                });
            }
            Err(err) => {
                self.apply_error("check", err);
                self.screen = AppScreen::Diagnostics;
                self.focus = AppFocus::DiagnosticsHistory;
            }
        }
        Ok(())
    }

    fn run_build(&mut self) -> Result<(), String> {
        if self.debug_session_blocks_project_change("build") {
            return Ok(());
        }
        let options = self.current_build_options();
        match actions::run_build_at(&self.project.summary.cwd, &options) {
            Ok(result) => {
                self.apply_build_result(&result);
                self.status.text = format!(
                    "build ok [{}]: {}",
                    result.target,
                    result.exe_path.display()
                );
                self.last_action = Some(ActionState {
                    name: "build".to_string(),
                    ok: true,
                    source: None,
                    summary: format!("Built executable {}", result.exe_path.display()),
                    detail: vec![
                        format!("target: {}", result.target),
                        format!("toolchain: {}", result.selected_compiler),
                        format!("c: {}", result.c_path.display()),
                        format!("debug map: {}", result.debug_map_path.display()),
                        format!("exe: {}", result.exe_path.display()),
                    ],
                });
                self.screen = AppScreen::BuildRun;
            }
            Err(err) => self.apply_error("build", err),
        }
        Ok(())
    }

    fn run_run(&mut self) -> Result<(), String> {
        if self.debug_session_blocks_project_change("run") {
            return Ok(());
        }
        let options = self.current_build_options();
        match actions::run_project_at(&self.project.summary.cwd, &options) {
            Ok(result) => {
                self.apply_run_result(&result);
                self.status.text = format!(
                    "run ok [{}]: {}",
                    result.build.target,
                    result.build.exe_path.display()
                );
                self.last_action = Some(ActionState {
                    name: "run".to_string(),
                    ok: true,
                    source: None,
                    summary: format!("Ran {}", result.build.exe_path.display()),
                    detail: vec![
                        format!("target: {}", result.build.target),
                        format!("exe: {}", result.build.exe_path.display()),
                        format!("status: {}", result.exit_status),
                        format!("stdout: {} byte(s)", result.stdout.len()),
                        format!("stderr: {} byte(s)", result.stderr.len()),
                    ],
                });
                self.screen = AppScreen::BuildRun;
            }
            Err(err) => self.apply_error("run", err),
        }
        Ok(())
    }

    fn debug_start_or_continue(&mut self) {
        if self.debug.run_state == DebugRunState::Stopped {
            self.debug_command(DebugCommand::Continue);
            return;
        }
        if matches!(
            self.debug.run_state,
            DebugRunState::Starting | DebugRunState::Running
        ) {
            self.status.text = "Debug program is already running.".to_string();
            return;
        }

        self.debug.run_state = DebugRunState::Starting;
        self.debug.current_stop = None;
        self.debug.selected_frame = 0;
        self.debug.output.clear();
        self.debug.exit_status = None;
        let options = DebugOptions {
            build: self.current_build_options(),
            breakpoints: Vec::new(),
            program_args: Vec::new(),
        };
        match actions::prepare_debug_at(&self.project.summary.cwd, &options) {
            Ok(prepared) => match start_debug_session(&prepared, DebugIoMode::Captured) {
                Ok(session) => {
                    self.last_build = Some(prepared.build.clone());
                    self.debug.prepared = Some(prepared);
                    self.debug.session = Some(session);
                    self.debug.run_state = DebugRunState::Running;
                    self.status.text =
                        "Debug session started; waiting for the first statement.".to_string();
                    self.screen = AppScreen::Debug;
                    self.focus = AppFocus::DebugFrames;
                }
                Err(error) => self.apply_debug_start_error(error),
            },
            Err(error) => self.apply_debug_start_error(error),
        }
    }

    fn apply_debug_start_error(&mut self, error: ActionError) {
        self.debug.run_state = DebugRunState::Failed;
        self.debug.output.push(error.to_string());
        self.status.text = "Debug session could not start.".to_string();
        self.apply_error("debug", error);
        self.screen = AppScreen::Debug;
        self.focus = AppFocus::DebugFrames;
    }

    fn debug_command(&mut self, command: DebugCommand) {
        if command == DebugCommand::Quit && self.debug.run_state != DebugRunState::Stopped {
            let Some(session) = &self.debug.session else {
                self.status.text = "No active debug session.".to_string();
                return;
            };
            match session.terminate() {
                Ok(()) => {
                    self.status.text = "Debug session is stopping.".to_string();
                }
                Err(message) => {
                    self.debug.run_state = DebugRunState::Failed;
                    self.debug.output.push(message.clone());
                    self.status.text = message;
                }
            }
            return;
        }
        if self.debug.run_state != DebugRunState::Stopped {
            self.status.text =
                "Debug commands are available while execution is stopped.".to_string();
            return;
        }
        let Some(session) = &self.debug.session else {
            self.status.text = "No active debug session.".to_string();
            return;
        };
        match session.command(command) {
            Ok(()) => {
                self.debug.run_state = DebugRunState::Running;
                self.status.text = match command {
                    DebugCommand::Continue => "Debug execution continued.".to_string(),
                    DebugCommand::Step => "Debug execution advanced by one statement.".to_string(),
                    DebugCommand::Quit => "Debug session is stopping.".to_string(),
                };
            }
            Err(message) => {
                self.debug.run_state = DebugRunState::Failed;
                self.debug.output.push(message.clone());
                self.status.text = message;
            }
        }
    }

    fn poll_debug_events(&mut self) {
        let mut pending = Vec::new();
        if let Some(session) = &self.debug.session {
            loop {
                match session.try_recv() {
                    Ok(event) => pending.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        let mut finished = false;
        for event in pending {
            finished |= matches!(event, DebugEvent::Exited { .. } | DebugEvent::Error(_));
            self.apply_debug_event(event);
        }
        if finished {
            self.debug.session = None;
        }
    }

    fn apply_debug_event(&mut self, event: DebugEvent) {
        match event {
            DebugEvent::Started => {
                self.debug.run_state = DebugRunState::Running;
            }
            DebugEvent::Stopped(stop) => {
                self.debug.run_state = DebugRunState::Stopped;
                self.debug.selected_frame = 0;
                self.status.text = format!(
                    "Stopped at {} on thread {}.",
                    self.debug_location(&stop.statement_id),
                    stop.thread_id
                );
                self.debug.current_stop = Some(stop);
            }
            DebugEvent::Stdout(line) => self.debug.output.push(format!("[out] {line}")),
            DebugEvent::Stderr(line) => self.debug.output.push(format!("[err] {line}")),
            DebugEvent::Exited { status, success } => {
                self.debug.run_state = if success {
                    DebugRunState::Exited
                } else {
                    DebugRunState::Failed
                };
                self.debug.exit_status = Some(status.clone());
                self.status.text = format!("Debug program exited: {status}");
            }
            DebugEvent::Error(message) => {
                self.debug.run_state = DebugRunState::Failed;
                self.debug.output.push(format!("[debug] {message}"));
                self.status.text = message;
            }
        }
    }

    fn debug_location(&self, statement_id: &str) -> String {
        self.debug
            .prepared
            .as_ref()
            .and_then(|prepared| {
                prepared
                    .build
                    .debug_map
                    .iter()
                    .find(|entry| entry.statement_id == statement_id)
            })
            .map(|entry| {
                format!(
                    "{}:{}:{}",
                    entry.source_path.display(),
                    entry.source_line,
                    entry.source_col
                )
            })
            .unwrap_or_else(|| statement_id.to_string())
    }

    fn run_format(&mut self) -> Result<(), String> {
        if self.debug_session_blocks_project_change("format") {
            return Ok(());
        }
        let options = FormatOptions {
            check_only: false,
            paths: Vec::new(),
        };
        match actions::run_format_at(&self.project.summary.cwd, &options) {
            Ok(result) => {
                self.refresh_project();
                let changed = result
                    .files
                    .iter()
                    .filter(|file| file.state == FormatState::Updated)
                    .count();
                self.status.text = format!(
                    "format ok: {} file(s), {} changed",
                    result.files.len(),
                    changed
                );
                self.last_action = Some(ActionState {
                    name: "format".to_string(),
                    ok: true,
                    source: None,
                    summary: self.status.text.clone(),
                    detail: result
                        .files
                        .iter()
                        .map(|file| match file.state {
                            FormatState::Updated => format!("formatted {}", file.path.display()),
                            FormatState::Unchanged => {
                                format!("already formatted {}", file.path.display())
                            }
                        })
                        .collect(),
                });
            }
            Err(err) => self.apply_error("format", err),
        }
        Ok(())
    }

    fn run_doctor(&mut self) -> Result<(), String> {
        match actions::run_doctor() {
            Ok(report) => {
                self.environment.report = Some(report.clone());
                self.screen = AppScreen::Doctor;
                self.status.text = if report.host_ready {
                    "doctor ok: host toolchain is ready".to_string()
                } else {
                    "doctor warning: host compiler not detected".to_string()
                };
                self.last_action = Some(ActionState {
                    name: "doctor".to_string(),
                    ok: report.host_ready,
                    source: None,
                    summary: self.status.text.clone(),
                    detail: report
                        .targets
                        .iter()
                        .map(|target| {
                            let label = if target.ready { "ready" } else { "missing" };
                            format!("{}: {}", target.triple, label)
                        })
                        .collect(),
                });
            }
            Err(err) => self.apply_error("doctor", err),
        }
        Ok(())
    }

    fn refresh_project(&mut self) {
        self.set_project_summary(actions::project_summary_at(&self.project.summary.cwd));
    }

    fn refresh_config(&mut self) {
        self.config.editing = false;
        self.config.edit_buffer.clear();
        self.config.manifest = actions::load_manifest_config(&self.project.summary.cwd).ok();
        self.config.saved_manifest = self.config.manifest.clone();
        self.config.selected = self
            .config
            .selected
            .min(ConfigField::all().len().saturating_sub(1));
    }

    fn start_config_edit(&mut self) {
        let Some(manifest) = &self.config.manifest else {
            self.status.text =
                "No manifest loaded. Initialize or open a Skadi project first.".to_string();
            return;
        };
        self.config.editing = true;
        self.focus = AppFocus::ConfigInput;
        self.config.edit_buffer = self.selected_config_value(manifest);
        self.status.text = format!(
            "Editing '{}'. Enter to apply, Esc to cancel.",
            self.selected_config_field().label()
        );
    }

    fn commit_config_edit(&mut self) {
        let field = self.selected_config_field();
        let value = self.config.edit_buffer.trim().to_string();
        if field.is_manifest_field() {
            if let Some(manifest) = &mut self.config.manifest {
                match field {
                    ConfigField::Name => manifest.name = value,
                    ConfigField::Version => manifest.version = value,
                    ConfigField::Edition => manifest.edition = value,
                    ConfigField::Entry => manifest.entry = value,
                    ConfigField::Target | ConfigField::Compiler => {}
                }
                self.config.editing = false;
                self.config.edit_buffer.clear();
                self.focus = AppFocus::ConfigFields;
                self.status.text = format!(
                    "Updated '{}' in memory. Press 's' to save Skadi.toml.",
                    field.label()
                );
            }
        } else {
            match field {
                ConfigField::Target => {
                    self.build_prefs.target = if value.is_empty() {
                        "host".to_string()
                    } else {
                        value
                    };
                }
                ConfigField::Compiler => {
                    self.build_prefs.compiler = if value.is_empty() { None } else { Some(value) };
                }
                ConfigField::Name
                | ConfigField::Version
                | ConfigField::Edition
                | ConfigField::Entry => {}
            }
            self.config.editing = false;
            self.config.edit_buffer.clear();
            self.focus = AppFocus::ConfigFields;
            self.status.text = format!("Updated session build preference '{}'.", field.label());
        }
    }

    fn save_config(&mut self) {
        let Some(manifest) = self.config.manifest.clone() else {
            self.status.text =
                "No manifest loaded. Initialize or open a Skadi project first.".to_string();
            return;
        };
        match actions::save_manifest_config(&self.project.summary.cwd, &manifest) {
            Ok(saved) => {
                self.config.manifest = Some(saved.clone());
                self.refresh_project();
                self.refresh_config();
                self.last_action = Some(ActionState {
                    name: "config save".to_string(),
                    ok: true,
                    source: None,
                    summary: format!("Saved {}", saved.manifest_path.display()),
                    detail: vec![
                        format!("name: {}", saved.name),
                        format!("version: {}", saved.version),
                        format!("edition: {}", saved.edition),
                        format!("entry: {}", saved.entry),
                    ],
                });
                self.status.text = "Config saved to Skadi.toml.".to_string();
            }
            Err(err) => self.apply_error("config save", err),
        }
    }

    fn generate_entry_file(&mut self) {
        match actions::ensure_project_entry_file(&self.project.summary.cwd) {
            Ok(path) => {
                self.refresh_project();
                self.refresh_config();
                self.last_action = Some(ActionState {
                    name: "entry generate".to_string(),
                    ok: true,
                    source: None,
                    summary: format!("Ensured entry file {}", path.display()),
                    detail: vec![
                        format!("root: {}", self.project.summary.cwd.display()),
                        format!("entry: {}", path.display()),
                    ],
                });
                self.status.text = format!("Entry file ready: {}", path.display());
            }
            Err(err) => self.apply_error("entry generate", err),
        }
    }

    fn open_project_path(&mut self, root: std::path::PathBuf) {
        if !root.exists() {
            self.apply_error(
                "open",
                ActionError::new(
                    FailureSource::Project,
                    format!("project path does not exist: {}", root.display()),
                ),
            );
            self.screen = AppScreen::Diagnostics;
            self.focus = AppFocus::DiagnosticsHistory;
            return;
        }
        if !root.is_dir() {
            self.apply_error(
                "open",
                ActionError::new(
                    FailureSource::Project,
                    format!("project path is not a directory: {}", root.display()),
                ),
            );
            self.screen = AppScreen::Diagnostics;
            self.focus = AppFocus::DiagnosticsHistory;
            return;
        }
        if let Some(session) = &self.debug.session {
            let _ = session.terminate();
        }
        self.debug = DebugState::default();
        self.set_project_summary(actions::project_summary_at(&root));
        self.bootstrap.mode = BootstrapMode::Idle;
        self.bootstrap.input.clear();
        self.focus = AppFocus::Tabs;
        self.screen = AppScreen::Home;
        self.last_build = None;
        self.last_run = None;
        self.status.text = if self.project.summary.loaded {
            format!("Opened project at {}", self.project.summary.cwd.display())
        } else {
            format!(
                "Switched to {}. No Skadi project found yet; use Bootstrap to init.",
                self.project.summary.cwd.display()
            )
        };
        self.last_action = Some(ActionState {
            name: "open".to_string(),
            ok: true,
            source: None,
            summary: if self.project.summary.loaded {
                format!(
                    "Opened project '{}'",
                    self.project
                        .summary
                        .name
                        .as_deref()
                        .unwrap_or("skadi_project")
                )
            } else {
                format!(
                    "Switched to '{}' with no Skadi.toml yet",
                    self.project.summary.cwd.display()
                )
            },
            detail: vec![
                format!("root: {}", self.project.summary.cwd.display()),
                format!("manifest: {}", self.project.summary.manifest.display()),
                format!(
                    "entry: {}",
                    self.project
                        .summary
                        .entry
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
            ],
        });
    }

    fn apply_check_result(&mut self, result: &CheckResult) {
        self.push_diagnostics_record(DiagnosticsRecord {
            action: "check".to_string(),
            ok: true,
            source: None,
            summary: if result.warnings.is_empty() {
                "Frontend check passed with no warnings".to_string()
            } else {
                format!(
                    "Frontend check passed with {} warning(s)",
                    result.warnings.len()
                )
            },
            diagnostics: result.warnings.clone(),
            analysis: result.analysis.clone(),
            detail: vec![format!("entry: {}", result.entry.display())],
        });
    }

    fn apply_build_result(&mut self, result: &BuildResult) {
        self.set_project_summary(result.project.clone());
        self.last_build = Some(result.clone());
        self.push_diagnostics_record(DiagnosticsRecord {
            action: "build".to_string(),
            ok: true,
            source: None,
            summary: if result.warnings.is_empty() {
                "Build passed frontend stage with no warnings".to_string()
            } else {
                format!(
                    "Build passed frontend stage with {} warning(s)",
                    result.warnings.len()
                )
            },
            diagnostics: result.warnings.clone(),
            analysis: result.analysis.clone(),
            detail: vec![
                format!("target: {}", result.target),
                format!(
                    "requested cc: {}",
                    result.requested_compiler.as_deref().unwrap_or("auto")
                ),
                format!("selected cc: {}", result.selected_compiler),
                format!(
                    "command: {}",
                    format_command_line(&result.selected_compiler, &result.compiler_args)
                ),
                format!("toolchain status: {}", result.toolchain_status),
                format!("exe: {}", result.exe_path.display()),
                format!("c: {}", result.c_path.display()),
                format!("debug map: {}", result.debug_map_path.display()),
            ],
        });
    }

    fn apply_run_result(&mut self, result: &RunResult) {
        self.apply_build_result(&result.build);
        self.last_run = Some(result.clone());
        self.push_diagnostics_record(DiagnosticsRecord {
            action: "run".to_string(),
            ok: true,
            source: None,
            summary: format!("Runtime execution completed with {}", result.exit_status),
            diagnostics: Vec::new(),
            analysis: Vec::new(),
            detail: vec![
                format!("exe: {}", result.build.exe_path.display()),
                format!("status: {}", result.exit_status),
                format!("stdout bytes: {}", result.stdout.len()),
                format!("stderr bytes: {}", result.stderr.len()),
            ],
        });
    }

    fn apply_error(&mut self, name: &str, err: ActionError) {
        self.status.text = format!("{} failed: {}", name, compact_message(&err.message));
        self.last_action = Some(ActionState {
            name: name.to_string(),
            ok: false,
            source: Some(err.source),
            summary: compact_message(&err.message),
            detail: err.message.lines().map(|line| line.to_string()).collect(),
        });
        self.push_diagnostics_record(DiagnosticsRecord {
            action: name.to_string(),
            ok: false,
            source: Some(err.source),
            summary: compact_message(&err.message),
            diagnostics: err.diagnostics.clone(),
            analysis: Vec::new(),
            detail: err.message.lines().map(|line| line.to_string()).collect(),
        });
    }

    fn requires_compact_layout(&self, area: Rect) -> bool {
        area.width < MIN_WIDTH || area.height < MIN_HEIGHT
    }

    fn push_diagnostics_record(&mut self, record: DiagnosticsRecord) {
        self.diagnostics.history.push(record);
        if self.diagnostics.history.len() > 16 {
            self.diagnostics.history.remove(0);
        }
        self.diagnostics.history_selected = self.diagnostics.history.len().saturating_sub(1);
        self.diagnostics.selected = 0;
        self.lifecycle.selected_subject = 0;
        self.lifecycle.selected_fact = 0;
    }

    fn current_record(&self) -> Option<&DiagnosticsRecord> {
        self.diagnostics
            .history
            .get(self.diagnostics.history_selected)
    }

    fn current_diagnostics(&self) -> &[DiagnosticSummary] {
        self.current_record()
            .map(|record| record.diagnostics.as_slice())
            .unwrap_or(&[])
    }

    fn current_analysis(&self) -> &[AnalysisFact] {
        self.current_record()
            .map(|record| record.analysis.as_slice())
            .unwrap_or(&[])
    }

    fn current_lifecycle_subjects(&self) -> Vec<LifecycleSubject<'_>> {
        lifecycle_subjects(self.current_analysis(), self.lifecycle.filter)
    }

    fn current_lifecycle_fact_count(&self) -> usize {
        self.current_lifecycle_subjects()
            .get(self.lifecycle.selected_subject)
            .map(|subject| subject.facts.len())
            .unwrap_or(0)
    }

    fn clamp_lifecycle_selection(&mut self) {
        let (selected_subject, fact_count) = {
            let subjects = self.current_lifecycle_subjects();
            let selected_subject = self
                .lifecycle
                .selected_subject
                .min(subjects.len().saturating_sub(1));
            let fact_count = subjects
                .get(selected_subject)
                .map(|subject| subject.facts.len())
                .unwrap_or(0);
            (selected_subject, fact_count)
        };
        self.lifecycle.selected_subject = selected_subject;
        self.lifecycle.selected_fact = self
            .lifecycle
            .selected_fact
            .min(fact_count.saturating_sub(1));
    }

    fn current_diagnostic_items_len(&self) -> usize {
        self.current_diagnostics().len() + self.current_analysis().len()
    }

    fn current_build_options(&self) -> BuildOptions {
        BuildOptions {
            target: self.build_prefs.target.clone(),
            cc: self.build_prefs.compiler.clone(),
        }
    }

    fn debug_session_blocks_project_change(&mut self, action: &str) -> bool {
        if self.debug.session.is_none() {
            return false;
        }
        self.status.text =
            format!("Stop the active debug session with F8 before running {action}.");
        self.screen = AppScreen::Debug;
        self.focus = AppFocus::DebugFrames;
        true
    }

    fn set_project_summary(&mut self, summary: ProjectSummary) {
        self.project = load_project_state(&summary);
        self.refresh_config();
    }

    fn selected_config_field(&self) -> ConfigField {
        ConfigField::all()[self.config.selected.min(ConfigField::all().len() - 1)]
    }

    fn selected_config_value(&self, manifest: &ManifestConfigResult) -> String {
        match self.selected_config_field() {
            ConfigField::Name => manifest.name.clone(),
            ConfigField::Version => manifest.version.clone(),
            ConfigField::Edition => manifest.edition.clone(),
            ConfigField::Entry => manifest.entry.clone(),
            ConfigField::Target => self.build_prefs.target.clone(),
            ConfigField::Compiler => self.build_prefs.compiler.clone().unwrap_or_default(),
        }
    }

    fn selected_entry_status(&self, manifest: &ManifestConfigResult) -> String {
        let path = self.project.summary.cwd.join(&manifest.entry);
        if path.exists() {
            format!("entry status: [ok] {}", path.display())
        } else {
            format!("entry status: [miss] {}", path.display())
        }
    }

    fn manifest_is_dirty(&self) -> bool {
        self.config.manifest != self.config.saved_manifest
    }
}

fn load_project_state(summary: &ProjectSummary) -> ProjectState {
    ProjectState {
        summary: summary.clone(),
        manifest_preview: load_manifest_preview(&summary.manifest),
        build_artifacts: load_build_artifacts(&summary.build_dir),
        manifest_exists: summary.manifest.exists(),
        entry_exists: summary
            .entry
            .as_ref()
            .map(|path| path.exists())
            .unwrap_or(false),
        build_dir_exists: summary.build_dir.exists(),
    }
}

fn lifecycle_subjects<'a>(
    facts: &'a [AnalysisFact],
    filter: LifecycleFilter,
) -> Vec<LifecycleSubject<'a>> {
    let mut subjects: Vec<LifecycleSubject<'a>> = Vec::new();
    for fact in facts {
        let (Some(id), Some(name), Some(kind)) = (
            fact.subject_id.as_deref(),
            fact.subject.as_deref(),
            fact.subject_kind,
        ) else {
            continue;
        };
        if let Some(subject) = subjects.iter_mut().find(|subject| subject.id == id) {
            if subject.kind == AnalysisSubjectKind::Resource
                && kind != AnalysisSubjectKind::Resource
            {
                subject.kind = kind;
            }
            subject.facts.push(fact);
        } else {
            subjects.push(LifecycleSubject {
                id,
                name,
                kind,
                facts: vec![fact],
            });
        }
    }
    subjects.retain(|subject| filter.accepts(subject.kind));
    subjects
}

fn load_manifest_preview(manifest: &Path) -> Vec<String> {
    match fs::read_to_string(manifest) {
        Ok(content) => content
            .lines()
            .take(10)
            .map(|line| line.to_string())
            .collect(),
        Err(_) => vec!["No Skadi.toml loaded yet.".to_string()],
    }
}

fn load_build_artifacts(build_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(build_dir) else {
        return vec!["No build artifacts yet.".to_string()];
    };

    let mut rows = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let size = entry.metadata().ok().map(|meta| meta.len()).unwrap_or(0);
            format!("{name} ({})", human_size(size))
        })
        .collect::<Vec<_>>();
    rows.sort();

    if rows.is_empty() {
        vec!["Build directory is empty.".to_string()]
    } else {
        rows.into_iter().take(8).collect()
    }
}

fn render(frame: &mut Frame<'_>, app: &mut App) {
    if app.requires_compact_layout(frame.area()) {
        render_small_terminal(frame, app);
        return;
    }

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_tabs(frame, areas[0], app);
    render_screen(frame, areas[1], app);
    render_status(frame, areas[2], app);
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let titles = [
        AppScreen::Home,
        AppScreen::Config,
        AppScreen::Diagnostics,
        AppScreen::Lifecycle,
        AppScreen::Debug,
        AppScreen::BuildRun,
        AppScreen::Doctor,
        AppScreen::Bootstrap,
        AppScreen::Help,
    ]
    .iter()
    .map(|screen| {
        let label = match screen {
            AppScreen::Config if app.manifest_is_dirty() => "Config*",
            _ => screen.title(),
        };
        Line::from(Span::raw(label))
    })
    .collect::<Vec<_>>();
    let selected = match app.screen {
        AppScreen::Home => 0,
        AppScreen::Config => 1,
        AppScreen::Diagnostics => 2,
        AppScreen::Lifecycle => 3,
        AppScreen::Debug => 4,
        AppScreen::BuildRun => 5,
        AppScreen::Doctor => 6,
        AppScreen::Bootstrap => 7,
        AppScreen::Help => 8,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("skadi tui"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .select(selected);
    frame.render_widget(tabs, area);
}

fn render_screen(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    match app.screen {
        AppScreen::Home => render_home(frame, area, app),
        AppScreen::Config => render_config(frame, area, app),
        AppScreen::Diagnostics => render_diagnostics(frame, area, app),
        AppScreen::Lifecycle => render_lifecycle(frame, area, app),
        AppScreen::Debug => render_debug(frame, area, app),
        AppScreen::BuildRun => render_build_run(frame, area, app),
        AppScreen::Doctor => render_doctor(frame, area, app),
        AppScreen::Bootstrap => render_bootstrap(frame, area, app),
        AppScreen::Help => render_help(frame, area),
    }
}

fn render_home(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(1)])
        .split(columns[0]);
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(1)])
        .split(columns[1]);

    let project = &app.project;
    let project_lines = vec![
        Line::from(format!("root: {}", project.summary.cwd.display())),
        Line::from(format!(
            "project: {}",
            project
                .summary
                .name
                .clone()
                .unwrap_or_else(|| "<not loaded>".to_string())
        )),
        Line::from(format!(
            "manifest: [{}] {}",
            if project.manifest_exists {
                "ok"
            } else {
                "miss"
            },
            project.summary.manifest.display()
        )),
        Line::from(format!(
            "entry: [{}] {}",
            if project.entry_exists { "ok" } else { "miss" },
            project
                .summary
                .entry
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<missing>".to_string())
        )),
        Line::from(format!(
            "build dir: [{}] {}",
            if project.build_dir_exists {
                "ok"
            } else {
                "miss"
            },
            project.summary.build_dir.display()
        )),
        Line::from(if project.summary.loaded {
            "status: ready".to_string()
        } else {
            "status: no Skadi.toml in selected directory".to_string()
        }),
        Line::from(format!(
            "last build: {}",
            app.last_build
                .as_ref()
                .map(|build| build.exe_path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string())
        )),
        Line::from(format!(
            "last run: {}",
            app.last_run
                .as_ref()
                .map(|run| run.exit_status.clone())
                .unwrap_or_else(|| "<none>".to_string())
        )),
        Line::from(format!(
            "build prefs: target={} cc={}",
            app.build_prefs.target,
            app.build_prefs.compiler.as_deref().unwrap_or("auto")
        )),
    ];
    let project_panel = Paragraph::new(project_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Project Dashboard"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(project_panel, left_rows[0]);

    let manifest_lines = project
        .manifest_preview
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect::<Vec<_>>();
    let manifest_panel = Paragraph::new(manifest_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Manifest Snapshot"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(manifest_panel, left_rows[1]);

    let mut right_lines = vec![
        Line::from("Actions"),
        Line::from("c check    b build    r run"),
        Line::from("f format   d doctor"),
        Line::from("p project  m config  e diagnostics"),
        Line::from("o open/switch project"),
        Line::from("Tab / Shift+Tab switch screens"),
        Line::from(""),
    ];
    if let Some(last) = &app.last_action {
        right_lines.push(Line::from(format!(
            "Last action: {} ({})",
            last.name,
            if last.ok { "ok" } else { "failed" }
        )));
        right_lines.push(Line::from(last.summary.clone()));
        if let Some(source) = last.source {
            right_lines.push(Line::from(format!("source: {}", source_label(source))));
        }
        for detail in last.detail.iter().take(3) {
            right_lines.push(Line::from(format!("  {detail}")));
        }
    } else {
        right_lines.push(Line::from("Last action: none yet"));
    }
    let status_panel = Paragraph::new(right_lines)
        .block(Block::default().borders(Borders::ALL).title("Workflow"))
        .wrap(Wrap { trim: false });
    frame.render_widget(status_panel, right_rows[0]);

    let artifact_lines = project
        .build_artifacts
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect::<Vec<_>>();
    let artifact_panel = Paragraph::new(artifact_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Build Artifacts"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(artifact_panel, right_rows[1]);
}

fn render_config(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let field_items = if let Some(manifest) = &app.config.manifest {
        ConfigField::all()
            .iter()
            .map(|field| {
                let value = match field {
                    ConfigField::Name => manifest.name.clone(),
                    ConfigField::Version => manifest.version.clone(),
                    ConfigField::Edition => manifest.edition.clone(),
                    ConfigField::Entry => manifest.entry.clone(),
                    ConfigField::Target => app.build_prefs.target.clone(),
                    ConfigField::Compiler => app
                        .build_prefs
                        .compiler
                        .clone()
                        .unwrap_or_else(|| "auto".to_string()),
                };
                let suffix = if field.is_manifest_field() {
                    ""
                } else {
                    " (session)"
                };
                ListItem::new(format!("{}{} = {}", field.label(), suffix, value))
            })
            .collect::<Vec<_>>()
    } else {
        vec![ListItem::new("No Skadi.toml loaded for the selected root.")]
    };

    let fields = List::new(field_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Manifest Fields"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    let mut state = ListState::default();
    if app.config.manifest.is_some() {
        state.select(Some(app.config.selected.min(ConfigField::all().len() - 1)));
    }
    frame.render_stateful_widget(fields, columns[0], &mut state);

    let detail_lines = if let Some(manifest) = &app.config.manifest {
        let selected = app.selected_config_field();
        let current_value = app.selected_config_value(manifest);
        let mut lines = vec![
            Line::from(format!("manifest: {}", manifest.manifest_path.display())),
            Line::from(format!(
                "manifest state: {}",
                if app.manifest_is_dirty() {
                    "modified"
                } else {
                    "clean"
                }
            )),
            Line::from(format!(
                "build prefs: target={} cc={}",
                app.build_prefs.target,
                app.build_prefs.compiler.as_deref().unwrap_or("auto")
            )),
            Line::from(format!("selected field: {}", selected.label())),
            Line::from(format!("current value: {}", current_value)),
            Line::from(app.selected_entry_status(manifest)),
            Line::from(""),
            Line::from("Enter  edit selected field"),
            Line::from("s      save Skadi.toml"),
            Line::from("g      generate missing entry file"),
            Line::from("Esc    cancel field edit"),
        ];
        if app.config.editing {
            lines.push(Line::from(""));
            lines.push(Line::from("Editing buffer"));
            lines.push(Line::from(format!("> {}", app.config.edit_buffer)));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from("Manifest Preview"));
            for line in [
                "[package]".to_string(),
                format!("name = \"{}\"", manifest.name),
                format!("version = \"{}\"", manifest.version),
                format!("edition = \"{}\"", manifest.edition),
                String::new(),
                "[build]".to_string(),
                format!("entry = \"{}\"", manifest.entry),
            ] {
                lines.push(Line::from(line));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Session Build Preferences"));
            lines.push(Line::from(format!("target = {}", app.build_prefs.target)));
            lines.push(Line::from(format!(
                "compiler = {}",
                app.build_prefs.compiler.as_deref().unwrap_or("auto")
            )));
        }
        lines
    } else {
        vec![
            Line::from("No manifest is currently loaded."),
            Line::from(""),
            Line::from("Open or initialize a Skadi project first."),
            Line::from("Use 'o' or the Bootstrap screen to switch roots."),
        ]
    };
    frame.render_widget(
        Paragraph::new(detail_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Config Editor"),
            )
            .wrap(Wrap { trim: false }),
        columns[1],
    );
}

fn render_diagnostics(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(28),
            Constraint::Percentage(42),
        ])
        .split(area);

    let history_items = if app.diagnostics.history.is_empty() {
        vec![ListItem::new(
            "No action history yet. Run check/build/format or trigger a failure.",
        )]
    } else {
        app.diagnostics
            .history
            .iter()
            .map(|record| {
                let counts = diagnostic_counts(&record.diagnostics);
                ListItem::new(format!(
                    "{} [{}] e:{} w:{} a:{} {}",
                    record.action,
                    if record.ok { "ok" } else { "fail" },
                    counts.errors,
                    counts.warnings,
                    record.analysis.len(),
                    compact_message(&record.summary)
                ))
            })
            .collect()
    };
    let history = List::new(history_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Action History"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    let mut history_state = ListState::default();
    if !app.diagnostics.history.is_empty() {
        history_state.select(Some(
            app.diagnostics
                .history_selected
                .min(app.diagnostics.history.len() - 1),
        ));
    }
    frame.render_stateful_widget(history, columns[0], &mut history_state);

    let mut diagnostic_items = app
        .current_diagnostics()
        .iter()
        .map(|diag| {
            let level = if diag.is_warning { "WARN" } else { "ERR" };
            let code = diag.code.as_deref().unwrap_or("-");
            let line = diag
                .line
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let col = diag
                .col
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            ListItem::new(format!(
                "[{}] {} {}:{} {}",
                level,
                code,
                line,
                col,
                compact_message(&diag.message)
            ))
        })
        .collect::<Vec<_>>();
    diagnostic_items.extend(app.current_analysis().iter().map(|fact| {
        ListItem::new(format!(
            "[{}] {} {}:{} {}",
            analysis_level_label(fact.level),
            fact.code,
            fact.line,
            fact.col,
            compact_message(&fact.summary)
        ))
    }));
    if diagnostic_items.is_empty() {
        diagnostic_items.push(ListItem::new(
            "No structured diagnostics or analysis facts for this action.",
        ));
    }
    let diagnostics_list = List::new(diagnostic_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Diagnostics / Analysis"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    let mut diag_state = ListState::default();
    if app.current_diagnostic_items_len() > 0 {
        diag_state.select(Some(
            app.diagnostics
                .selected
                .min(app.current_diagnostic_items_len() - 1),
        ));
    }
    frame.render_stateful_widget(diagnostics_list, columns[1], &mut diag_state);

    let detail_lines = if let Some(record) = app.current_record() {
        if let Some(diag) = record.diagnostics.get(app.diagnostics.selected) {
            let mut lines = vec![
                Line::from(format!(
                    "{} {}",
                    if diag.is_warning { "Warning" } else { "Error" },
                    diag.code.as_deref().unwrap_or("-")
                )),
                Line::from(format!("action: {}", record.action)),
                Line::from(format!(
                    "result: {}",
                    if record.ok { "ok" } else { "failed" }
                )),
                Line::from(format!(
                    "source: {}",
                    record
                        .source
                        .map(source_label)
                        .unwrap_or("frontend diagnostics")
                )),
                Line::from(format!("stage: {}", diag.stage)),
                Line::from(format!(
                    "location: line {}, col {}",
                    diag.line
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    diag.col
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "-".to_string())
                )),
                Line::from(""),
                Line::from(diag.message.clone()),
            ];
            if !record.detail.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from("Action context"));
                for detail in record.detail.iter().take(6) {
                    lines.push(Line::from(detail.clone()));
                }
            }
            lines
        } else if let Some(fact) = record.analysis.get(
            app.diagnostics
                .selected
                .saturating_sub(record.diagnostics.len()),
        ) {
            let mut lines = vec![
                Line::from(format!(
                    "Analysis {} {}",
                    analysis_level_label(fact.level),
                    fact.code
                )),
                Line::from(format!("action: {}", record.action)),
                Line::from(format!("context: {}", fact.context)),
                Line::from(format!("location: line {}, col {}", fact.line, fact.col)),
                Line::from(""),
                Line::from(fact.summary.clone()),
                Line::from(""),
                Line::from("Why it matters"),
                Line::from(fact.explanation.clone()),
                Line::from(""),
                Line::from("Next action"),
                Line::from(fact.action.clone()),
            ];
            if let Some(subject) = &fact.subject {
                lines.push(Line::from(""));
                lines.push(Line::from(format!("Lifecycle: {subject}")));
                for related in record
                    .analysis
                    .iter()
                    .filter(|candidate| candidate.subject.as_ref() == Some(subject))
                    .take(8)
                {
                    lines.push(Line::from(format!(
                        "line {}  {}  {}",
                        related.line, related.code, related.summary
                    )));
                }
            }
            lines
        } else {
            let counts = diagnostic_counts(&record.diagnostics);
            let mut lines = vec![
                Line::from(format!("action: {}", record.action)),
                Line::from(format!(
                    "result: {}",
                    if record.ok { "ok" } else { "failed" }
                )),
                Line::from(format!(
                    "source: {}",
                    record.source.map(source_label).unwrap_or("workflow")
                )),
                Line::from(format!("errors: {}", counts.errors)),
                Line::from(format!("warnings: {}", counts.warnings)),
                Line::from(format!("analysis facts: {}", record.analysis.len())),
                Line::from(""),
                Line::from(record.summary.clone()),
            ];
            if !record.detail.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from("Raw details"));
                for detail in record.detail.iter().take(8) {
                    lines.push(Line::from(detail.clone()));
                }
            }
            lines
        }
    } else {
        vec![
            Line::from("No diagnostics workbench data yet."),
            Line::from(""),
            Line::from("Use c/b/r/f to populate action history."),
        ]
    };
    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Detail / Context"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, columns[2]);
}

fn render_lifecycle(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(34),
            Constraint::Percentage(38),
        ])
        .split(rows[1]);

    let filters = format!(
        "1 All  2 Tasks  3 Channels  4 Memory  5 Resources   active=[{}]   Left/Right panes, j/k navigate",
        app.lifecycle.filter.label()
    );
    frame.render_widget(
        Paragraph::new(filters).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Lifecycle Workspace"),
        ),
        rows[0],
    );

    let subjects = app.current_lifecycle_subjects();
    let subject_items = if subjects.is_empty() {
        vec![ListItem::new(
            "No lifecycle subjects. Run check/build on a project that uses owned resources.",
        )]
    } else {
        subjects
            .iter()
            .map(|subject| {
                let attention = subject
                    .facts
                    .iter()
                    .filter(|fact| fact.level == AnalysisFactLevel::Attention)
                    .count();
                ListItem::new(format!(
                    "[{}] {}  events:{} check:{}",
                    subject_kind_marker(subject.kind),
                    subject.name,
                    subject.facts.len(),
                    attention
                ))
            })
            .collect()
    };
    let subject_title = if app.focus == AppFocus::LifecycleSubjects {
        "Subjects [focused]"
    } else {
        "Subjects"
    };
    let subject_list = List::new(subject_items)
        .block(Block::default().borders(Borders::ALL).title(subject_title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    let mut subject_state = ListState::default();
    if !subjects.is_empty() {
        subject_state.select(Some(app.lifecycle.selected_subject.min(subjects.len() - 1)));
    }
    frame.render_stateful_widget(subject_list, columns[0], &mut subject_state);

    let selected_subject = subjects.get(app.lifecycle.selected_subject);
    let timeline_items = selected_subject
        .map(|subject| {
            subject
                .facts
                .iter()
                .map(|fact| {
                    ListItem::new(format!(
                        "[{}] {} {}:{} {}",
                        analysis_level_label(fact.level),
                        fact.code,
                        fact.span.start_line,
                        fact.span.start_col,
                        compact_message(&fact.summary)
                    ))
                })
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec![ListItem::new("Select a lifecycle subject.")]);
    let timeline_title = if app.focus == AppFocus::LifecycleFacts {
        "Timeline [focused]"
    } else {
        "Timeline"
    };
    let timeline = List::new(timeline_items)
        .block(Block::default().borders(Borders::ALL).title(timeline_title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    let mut timeline_state = ListState::default();
    if let Some(subject) = selected_subject
        && !subject.facts.is_empty()
    {
        timeline_state.select(Some(
            app.lifecycle.selected_fact.min(subject.facts.len() - 1),
        ));
    }
    frame.render_stateful_widget(timeline, columns[1], &mut timeline_state);

    let detail_lines = selected_subject
        .and_then(|subject| {
            subject
                .facts
                .get(app.lifecycle.selected_fact)
                .map(|fact| (subject, *fact))
        })
        .map(|(subject, fact)| {
            vec![
                Line::from(format!(
                    "[{}] {}",
                    subject_kind_marker(subject.kind),
                    subject.name
                )),
                Line::from(format!("subject id: {}", subject.id)),
                Line::from(format!("fact id: {}", fact.id)),
                Line::from(format!("event: {} ({})", fact.kind_name(), fact.code)),
                Line::from(format!("context: {}", fact.context)),
                Line::from(format!(
                    "source: {}:{}-{}:{}",
                    fact.span.start_line,
                    fact.span.start_col,
                    fact.span.end_line,
                    fact.span.end_col
                )),
                Line::from(""),
                Line::from(fact.summary.clone()),
                Line::from(""),
                Line::from("Why it matters"),
                Line::from(fact.explanation.clone()),
                Line::from(""),
                Line::from("Next action"),
                Line::from(fact.action.clone()),
            ]
        })
        .unwrap_or_else(|| {
            vec![
                Line::from("No lifecycle event selected."),
                Line::from(""),
                Line::from("Run check or build, then choose a subject."),
            ]
        });
    frame.render_widget(
        Paragraph::new(detail_lines)
            .block(Block::default().borders(Borders::ALL).title("Explain"))
            .wrap(Wrap { trim: false }),
        columns[2],
    );
}

fn render_debug(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(rows[1]);

    let stop = app.debug.current_stop.as_ref();
    let session_lines = vec![
        Line::from(format!("state: {}", app.debug.run_state.label())),
        Line::from(format!(
            "location: {}",
            stop.map(|value| app.debug_location(&value.statement_id))
                .unwrap_or_else(|| "<not stopped>".to_string())
        )),
        Line::from(format!(
            "thread: {}",
            stop.map(|value| value.thread_id.to_string())
                .unwrap_or_else(|| "-".to_string())
        )),
        Line::from(format!(
            "exit: {}",
            app.debug.exit_status.as_deref().unwrap_or("-")
        )),
        Line::from("F5/Enter start or continue | F10 step | F8 stop"),
    ];
    frame.render_widget(
        Paragraph::new(session_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Debug Session"),
            )
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    let frame_items = stop
        .map(|value| {
            value
                .frames
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    ListItem::new(format!(
                        "#{index} {}  {}",
                        item.function,
                        app.debug_location(&item.statement_id)
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut frame_state = ListState::default();
    if !frame_items.is_empty() {
        frame_state.select(Some(
            app.debug
                .selected_frame
                .min(frame_items.len().saturating_sub(1)),
        ));
    }
    frame.render_stateful_widget(
        List::new(frame_items)
            .block(Block::default().borders(Borders::ALL).title("Call Stack"))
            .highlight_symbol("> ")
            .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
        columns[0],
        &mut frame_state,
    );

    let local_lines = stop
        .and_then(|value| value.frames.get(app.debug.selected_frame))
        .map(|selected| {
            let mut lines = vec![Line::from(format!("frame: {}", selected.function))];
            if selected.locals.is_empty() {
                lines.push(Line::from("<no scalar locals captured yet>"));
            } else {
                lines.extend(selected.locals.iter().map(|local| {
                    Line::from(format!(
                        "{}: {} = {}",
                        local.name, local.type_name, local.value
                    ))
                }));
            }
            lines
        })
        .unwrap_or_else(|| {
            vec![
                Line::from("No stopped frame."),
                Line::from("Press F5 or Enter to build and start debugging."),
            ]
        });
    frame.render_widget(
        Paragraph::new(local_lines)
            .block(Block::default().borders(Borders::ALL).title("Locals"))
            .wrap(Wrap { trim: false }),
        columns[1],
    );

    let output = if app.debug.output.is_empty() {
        vec![Line::from("Program output will appear here.")]
    } else {
        app.debug
            .output
            .iter()
            .rev()
            .take(rows[2].height.saturating_sub(2) as usize)
            .rev()
            .cloned()
            .map(Line::from)
            .collect()
    };
    frame.render_widget(
        Paragraph::new(output)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Program Output [out/error markers]"),
            )
            .wrap(Wrap { trim: false }),
        rows[2],
    );
}

fn render_build_run(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(1)])
        .split(area);
    let summary_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);
    let console_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let build_lines = if let Some(build) = &app.last_build {
        vec![
            Line::from(format!("target: {}", build.target)),
            Line::from(format!(
                "requested cc: {}",
                build.requested_compiler.as_deref().unwrap_or("auto")
            )),
            Line::from(format!("selected cc: {}", build.selected_compiler)),
            Line::from(format!("status: {}", build.toolchain_status)),
            Line::from(format!("c path: {}", build.c_path.display())),
            Line::from(format!("debug map: {}", build.debug_map_path.display())),
            Line::from(format!("exe path: {}", build.exe_path.display())),
            Line::from(format!("warnings: {}", build.warnings.len())),
            Line::from("Press 'b' to rebuild the current project."),
        ]
    } else {
        vec![
            Line::from("No build has been run yet."),
            Line::from("Press 'b' to build the current project."),
        ]
    };
    frame.render_widget(
        Paragraph::new(build_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Build Summary"),
            )
            .wrap(Wrap { trim: false }),
        summary_columns[0],
    );

    let run_lines = if let Some(run) = &app.last_run {
        vec![
            Line::from(format!("exe: {}", run.build.exe_path.display())),
            Line::from(format!("status: {}", run.exit_status)),
            Line::from(format!("stdout bytes: {}", run.stdout.len())),
            Line::from(format!("stderr bytes: {}", run.stderr.len())),
            Line::from("Press 'r' to rebuild and run again."),
        ]
    } else {
        vec![
            Line::from("No run has been executed yet."),
            Line::from("Press 'r' to build and run the current project."),
        ]
    };
    frame.render_widget(
        Paragraph::new(run_lines)
            .block(Block::default().borders(Borders::ALL).title("Run Summary"))
            .wrap(Wrap { trim: false }),
        summary_columns[1],
    );

    let build_console_lines = if let Some(build) = &app.last_build {
        console_panel_lines(
            "toolchain",
            &format_command_line(&build.selected_compiler, &build.compiler_args),
            &build.toolchain_stdout,
            &build.toolchain_stderr,
        )
    } else {
        vec![
            Line::from("No compiler invocation captured yet."),
            Line::from("Build the project to inspect toolchain context."),
        ]
    };
    frame.render_widget(
        Paragraph::new(build_console_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Build Console / Toolchain"),
            )
            .wrap(Wrap { trim: false }),
        console_columns[0],
    );

    let runtime_console_lines = if let Some(run) = &app.last_run {
        console_panel_lines(
            "runtime",
            &run.build.exe_path.display().to_string(),
            &run.stdout,
            &run.stderr,
        )
    } else {
        vec![
            Line::from("No runtime output captured yet."),
            Line::from("Run the project to inspect stdout/stderr."),
        ]
    };
    frame.render_widget(
        Paragraph::new(runtime_console_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Run Console / Runtime"),
            )
            .wrap(Wrap { trim: false }),
        console_columns[1],
    );
}

fn render_doctor(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let report = app
        .environment
        .report
        .clone()
        .unwrap_or_else(|| DoctorReport {
            host_candidates: Vec::new(),
            host_ready: false,
            host_install_hint: "Run 'd' to inspect toolchains.".to_string(),
            shell_probe_hint: String::new(),
            targets: Vec::new(),
        });
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let mut host_lines = vec![Line::from(if report.host_ready {
        "host status: ready".to_string()
    } else {
        "host status: not ready".to_string()
    })];
    for host in &report.host_candidates {
        host_lines.push(Line::from(format!(
            "[{}] {}",
            if host.available { "ok" } else { "miss" },
            host.program
        )));
    }
    host_lines.push(Line::from(""));
    host_lines.push(Line::from(format!("install: {}", report.host_install_hint)));
    if !report.shell_probe_hint.is_empty() {
        host_lines.push(Line::from(format!("probe: {}", report.shell_probe_hint)));
    }
    frame.render_widget(
        Paragraph::new(host_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Host Toolchain"),
            )
            .wrap(Wrap { trim: false }),
        columns[0],
    );

    let mut target_lines = Vec::new();
    for target in &report.targets {
        target_lines.push(Line::from(format!(
            "{} [{}]",
            target.triple,
            if target.ready { "ready" } else { "missing" }
        )));
        for status in &target.statuses {
            target_lines.push(Line::from(format!(
                "  [{}] {}",
                if status.available { "ok" } else { "miss" },
                status.program
            )));
        }
        target_lines.push(Line::from(format!("  hint: {}", target.hint)));
        target_lines.push(Line::from(""));
    }
    if target_lines.is_empty() {
        target_lines.push(Line::from(
            "Run 'd' to refresh target toolchain availability.",
        ));
    }
    frame.render_widget(
        Paragraph::new(target_lines)
            .block(Block::default().borders(Borders::ALL).title("Targets"))
            .wrap(Wrap { trim: false }),
        columns[1],
    );
}

fn render_bootstrap(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = match app.bootstrap.mode {
        BootstrapMode::Idle => vec![
            Line::from("o  Open or switch to another project directory"),
            Line::from("n  Start a new project in a new directory"),
            Line::from("i  Initialize the selected directory"),
            Line::from(""),
            Line::from(format!(
                "Selected root: {}",
                app.project.summary.cwd.display()
            )),
            Line::from("Use this screen to switch roots or bootstrap a directory."),
        ],
        BootstrapMode::NewProject => vec![
            Line::from("New project name"),
            Line::from(format!("> {}", app.bootstrap.input)),
            Line::from(""),
            Line::from(format!(
                "Parent directory: {}",
                app.project.summary.cwd.display()
            )),
            Line::from("Enter to create and open, Esc to cancel."),
        ],
        BootstrapMode::OpenProject => vec![
            Line::from("Open project directory"),
            Line::from(format!("> {}", app.bootstrap.input)),
            Line::from(""),
            Line::from("Enter an absolute path or a path relative to the selected root."),
            Line::from("Enter to switch, Esc to cancel."),
        ],
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Bootstrap"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let help = vec![
        Line::from("Navigation"),
        Line::from("q quit"),
        Line::from("Tab / Shift+Tab switch screens"),
        Line::from("m open config editor"),
        Line::from("Left/Right switch diagnostics or lifecycle panes"),
        Line::from("j/k or arrows navigate diagnostics, lifecycle, and history"),
        Line::from("1-5 filter lifecycle subjects"),
        Line::from(""),
        Line::from("Actions"),
        Line::from("c check"),
        Line::from("b build"),
        Line::from("r run"),
        Line::from("f format"),
        Line::from("d doctor"),
        Line::from("p project"),
        Line::from("m config"),
        Line::from("e diagnostics"),
        Line::from("l lifecycle analysis"),
        Line::from("x debug workspace"),
        Line::from("h help"),
        Line::from("F5 / Enter start or continue debug execution"),
        Line::from("F10 step one Skadi statement while stopped"),
        Line::from("F8 stop a debug session"),
        Line::from("o/n/i bootstrap actions in Bootstrap view"),
        Line::from("Enter edit config field"),
        Line::from("s save Skadi.toml from Config view"),
        Line::from("g generate missing entry file from Config view"),
        Line::from(""),
        Line::from("Current limitations"),
        Line::from("No showcase browser yet."),
        Line::from("No async background task runner yet."),
        Line::from("Debugged programs receive EOF from input() in TUI mode."),
        Line::from("Use CLI debug for interactive program stdin and source breakpoints."),
        Line::from("Regular commands remain the canonical CI/scripting path."),
    ];
    frame.render_widget(
        Paragraph::new(help)
            .block(Block::default().borders(Borders::ALL).title("Keybindings"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let status = format!(
        "{} | screen={} | root={} | manifest={} | target={} | cc={}",
        app.status.text,
        app.screen.title(),
        compact_message(&app.project.summary.cwd.display().to_string()),
        if app.manifest_is_dirty() {
            "modified"
        } else {
            "clean"
        },
        app.build_prefs.target,
        app.build_prefs.compiler.as_deref().unwrap_or("auto"),
    );
    let text = Paragraph::new(status).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(text, area);
}

fn render_small_terminal(frame: &mut Frame<'_>, app: &App) {
    let lines = vec![
        Line::from("Terminal too small for skadi tui."),
        Line::from(format!("Minimum size: {}x{}", MIN_WIDTH, MIN_HEIGHT)),
        Line::from("Resize the terminal or use the plain CLI commands."),
        Line::from(""),
        Line::from(format!("Current screen: {}", app.screen.title())),
        Line::from("Press q to quit."),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("skadi tui"))
            .wrap(Wrap { trim: false }),
        frame.area(),
    );
}

fn format_diagnostic_line(diag: &DiagnosticSummary) -> String {
    let level = if diag.is_warning { "warning" } else { "error" };
    let code = diag.code.as_deref().unwrap_or("-");
    format!("{level} {code}: {}", diag.message)
}

fn format_command_line(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    }
}

fn console_panel_lines(
    label: &str,
    command_or_target: &str,
    stdout: &str,
    stderr: &str,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!("{label}: {command_or_target}"))];

    if stdout.trim().is_empty() {
        lines.push(Line::from("stdout: <empty>"));
    } else {
        lines.push(Line::from("stdout:"));
        for line in stdout.lines() {
            lines.push(Line::from(format!("  {line}")));
        }
    }

    if stderr.trim().is_empty() {
        lines.push(Line::from("stderr: <empty>"));
    } else {
        lines.push(Line::from("stderr:"));
        for line in stderr.lines() {
            lines.push(Line::from(format!("  {line}")));
        }
    }

    lines
}

fn source_label(source: FailureSource) -> &'static str {
    match source {
        FailureSource::Frontend => "frontend",
        FailureSource::Toolchain => "toolchain",
        FailureSource::Runtime => "runtime",
        FailureSource::Project => "project",
        FailureSource::Io => "io",
        FailureSource::Usage => "usage",
    }
}

fn analysis_level_label(level: AnalysisFactLevel) -> &'static str {
    match level {
        AnalysisFactLevel::Info => "INFO",
        AnalysisFactLevel::Attention => "CHECK",
    }
}

fn subject_kind_marker(kind: AnalysisSubjectKind) -> &'static str {
    match kind {
        AnalysisSubjectKind::Task => "TASK",
        AnalysisSubjectKind::Channel => "CHAN",
        AnalysisSubjectKind::Memory => "MEM",
        AnalysisSubjectKind::Resource => "RES",
    }
}

fn compact_message(message: &str) -> String {
    let first = message.lines().next().unwrap_or(message).trim();
    if first.chars().count() <= 96 {
        first.to_string()
    } else {
        let mut out = String::new();
        for (idx, ch) in first.chars().enumerate() {
            if idx >= 93 {
                break;
            }
            out.push(ch);
        }
        out.push_str("...");
        out
    }
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

struct DiagnosticCounts {
    errors: usize,
    warnings: usize,
}

fn diagnostic_counts(items: &[DiagnosticSummary]) -> DiagnosticCounts {
    let warnings = items.iter().filter(|diag| diag.is_warning).count();
    DiagnosticCounts {
        errors: items.len().saturating_sub(warnings),
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{App, AppScreen, DiagnosticsRecord, MIN_HEIGHT, MIN_WIDTH};
    use crate::actions::{
        ActionError, AnalysisFact, AnalysisFactLevel, AnalysisSubjectKind, DiagnosticSummary,
        FailureSource,
    };
    use crate::debug_session::{DebugEvent, DebugFrame, DebugLocal, DebugStop};
    use crate::project::init_project;
    use ratatui::{Terminal, backend::TestBackend};
    use v01::analysis::AnalysisFactKind;

    fn unique_temp_dir(stem: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_millis();
        let dir = std::env::temp_dir().join(format!("skadi_tui_{stem}_{stamp}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn app_starts_on_home_screen() {
        let app = App::new();
        assert_eq!(app.screen, AppScreen::Home);
        assert!(!app.should_quit);
    }

    #[test]
    fn tab_switches_screens() {
        let mut app = App::new();
        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ))
        .expect("tab should work");
        assert_eq!(app.screen, AppScreen::Config);
    }

    #[test]
    fn debug_hotkey_opens_workspace() {
        let mut app = App::new();
        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('x'),
        ))
        .expect("debug hotkey should work");
        assert_eq!(app.screen, AppScreen::Debug);
        assert_eq!(app.focus, super::AppFocus::DebugFrames);
    }

    #[test]
    fn debug_stop_maps_frames_locals_and_renders_workspace() {
        let mut app = App::new();
        app.screen = AppScreen::Debug;
        app.apply_debug_event(DebugEvent::Stopped(DebugStop {
            thread_id: 17,
            statement_id: "SK-STMT@2:5#1".to_string(),
            frames: vec![
                DebugFrame {
                    function: "worker".to_string(),
                    statement_id: "SK-STMT@2:5#1".to_string(),
                    locals: vec![DebugLocal {
                        name: "value".to_string(),
                        type_name: "Int".to_string(),
                        value: "42".to_string(),
                    }],
                },
                DebugFrame {
                    function: "<main>".to_string(),
                    statement_id: "SK-STMT@8:1#1".to_string(),
                    locals: Vec::new(),
                },
            ],
        }));

        assert_eq!(app.debug.run_state, super::DebugRunState::Stopped);
        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ))
        .expect("frame navigation should work");
        assert_eq!(app.debug.selected_frame, 1);
        app.debug.selected_frame = 0;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| super::render(frame, &mut app))
            .expect("render debug workspace");
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Debug Session"));
        assert!(rendered.contains("Call Stack"));
        assert!(rendered.contains("worker"));
        assert!(rendered.contains("value: Int = 42"));
        assert!(rendered.contains("thread: 17"));
    }

    #[test]
    fn diagnostics_navigation_moves_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Diagnostics;
        app.apply_error(
            "check",
            ActionError::new(
                FailureSource::Frontend,
                "Semantic error at line 1, col 2 [SC-SEM-020]: undefined symbol 'x'\nSemantic error at line 2, col 3 [SC-SEM-020]: undefined symbol 'y'",
            ),
        );
        app.focus = super::AppFocus::DiagnosticsList;
        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ))
        .expect("down should work");
        assert_eq!(app.diagnostics.selected, 1);
    }

    #[test]
    fn analysis_facts_share_the_diagnostics_navigation_model() {
        let mut app = App::new();
        app.screen = AppScreen::Diagnostics;
        app.push_diagnostics_record(DiagnosticsRecord {
            action: "check".to_string(),
            ok: true,
            source: None,
            summary: "check passed".to_string(),
            diagnostics: vec![DiagnosticSummary {
                stage: "Semantic".to_string(),
                code: Some("SC-SEM-040".to_string()),
                line: Some(1),
                col: Some(1),
                message: "style warning".to_string(),
                is_warning: true,
            }],
            analysis: vec![AnalysisFact {
                id: "SC-AN-102@4:9#1".to_string(),
                kind: AnalysisFactKind::BlockingChannelReceive,
                level: AnalysisFactLevel::Attention,
                code: "SC-AN-102",
                line: 4,
                col: 9,
                span: v01::analysis::AnalysisSourceSpan {
                    start_line: 4,
                    start_col: 9,
                    end_line: 4,
                    end_col: 9,
                },
                context: "task entry 'worker'".to_string(),
                subject: Some("jobs".to_string()),
                subject_id: Some("task entry 'worker'::jobs".to_string()),
                subject_kind: Some(AnalysisSubjectKind::Channel),
                summary: "'jobs.receive' may block".to_string(),
                explanation: "cancellation needs an explicit path".to_string(),
                action: "attach on error".to_string(),
            }],
            detail: Vec::new(),
        });
        app.focus = super::AppFocus::DiagnosticsList;

        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ))
        .expect("down should select analysis fact");

        assert_eq!(app.diagnostics.selected, 1);
        assert_eq!(app.current_analysis()[0].code, "SC-AN-102");
    }

    #[test]
    fn diagnostics_detail_renders_timed_task_lifecycle_chain() {
        let mut app = App::new();
        app.screen = AppScreen::Diagnostics;
        app.push_diagnostics_record(DiagnosticsRecord {
            action: "check".to_string(),
            ok: true,
            source: None,
            summary: "check passed".to_string(),
            diagnostics: Vec::new(),
            analysis: vec![
                AnalysisFact {
                    id: "SC-AN-311@1:1#1".to_string(),
                    kind: AnalysisFactKind::TaskLifecycle,
                    level: AnalysisFactLevel::Info,
                    code: "SC-AN-311",
                    line: 1,
                    col: 1,
                    span: v01::analysis::AnalysisSourceSpan {
                        start_line: 1,
                        start_col: 1,
                        end_line: 1,
                        end_col: 1,
                    },
                    context: "top-level workflow".to_string(),
                    subject: Some("worker_task".to_string()),
                    subject_id: Some("top-level workflow::worker_task".to_string()),
                    subject_kind: Some(AnalysisSubjectKind::Task),
                    summary: "task 'worker_task' starts".to_string(),
                    explanation: "task owner created".to_string(),
                    action: "follow lifecycle".to_string(),
                },
                AnalysisFact {
                    id: "SC-AN-314@8:1#1".to_string(),
                    kind: AnalysisFactKind::TimedTaskWait,
                    level: AnalysisFactLevel::Info,
                    code: "SC-AN-314",
                    line: 8,
                    col: 1,
                    span: v01::analysis::AnalysisSourceSpan {
                        start_line: 8,
                        start_col: 1,
                        end_line: 8,
                        end_col: 1,
                    },
                    context: "top-level workflow".to_string(),
                    subject: Some("worker_task".to_string()),
                    subject_id: Some("top-level workflow::worker_task".to_string()),
                    subject_kind: Some(AnalysisSubjectKind::Task),
                    summary: "task 'worker_task' is waited with a deadline".to_string(),
                    explanation: "timeout preserves the handle".to_string(),
                    action: "finish timeout cleanup".to_string(),
                },
            ],
            detail: Vec::new(),
        });

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| super::render(frame, &mut app))
            .expect("render lifecycle detail");
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Lifecycle: worker_task"));
        assert!(rendered.contains("SC-AN-314"));

        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('l'),
        ))
        .expect("lifecycle hotkey");
        assert_eq!(app.screen, AppScreen::Lifecycle);
        assert_eq!(app.current_lifecycle_subjects().len(), 1);

        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('3'),
        ))
        .expect("channel filter");
        assert!(app.current_lifecycle_subjects().is_empty());
        app.handle_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('2'),
        ))
        .expect("task filter");
        assert_eq!(app.current_lifecycle_subjects().len(), 1);

        terminal
            .draw(|frame| super::render(frame, &mut app))
            .expect("render lifecycle workspace");
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Lifecycle Workspace"));
        assert!(rendered.contains("TASK"));
        assert!(rendered.contains("SC-AN-311"));
        assert!(rendered.contains("fact id:"));
    }

    #[test]
    fn diagnostics_history_tracks_latest_record() {
        let mut app = App::new();
        app.apply_error(
            "check",
            ActionError::new(
                FailureSource::Frontend,
                "Semantic error at line 1, col 2 [SC-SEM-020]: undefined symbol 'x'",
            ),
        );
        app.apply_error(
            "build",
            ActionError::new(
                FailureSource::Toolchain,
                "C toolchain error: failed to run gcc",
            ),
        );
        assert_eq!(app.diagnostics.history.len(), 2);
        assert_eq!(app.diagnostics.history_selected, 1);
        assert_eq!(
            app.current_record().map(|record| record.action.as_str()),
            Some("build")
        );
    }

    #[test]
    fn compact_layout_detects_small_terminal() {
        let app = App::new();
        assert!(app.requires_compact_layout(ratatui::layout::Rect::new(
            0,
            0,
            MIN_WIDTH - 1,
            MIN_HEIGHT
        )));
    }

    #[test]
    fn open_project_path_switches_selected_root() {
        let temp = unique_temp_dir("open_project");
        init_project(&temp).expect("project should init");

        let mut app = App::new();
        app.open_project_path(temp.clone());

        assert_eq!(app.project.summary.cwd, temp);
        assert!(app.project.summary.loaded);
        assert_eq!(
            app.last_action.as_ref().map(|action| action.name.as_str()),
            Some("open")
        );

        let _ = fs::remove_dir_all(app.project.summary.cwd.clone());
    }

    #[test]
    fn open_project_path_rejects_missing_directory() {
        let temp = unique_temp_dir("missing_project");
        let missing = temp.join("does_not_exist");

        let mut app = App::new();
        app.open_project_path(missing);

        assert_eq!(app.screen, AppScreen::Diagnostics);
        assert_eq!(
            app.last_action.as_ref().map(|action| action.name.as_str()),
            Some("open")
        );
        assert!(
            app.last_action
                .as_ref()
                .map(|action| action.summary.contains("does not exist"))
                .unwrap_or(false)
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn project_workbench_tracks_manifest_and_artifacts() {
        let temp = unique_temp_dir("workbench");
        init_project(&temp).expect("project should init");
        let build = temp.join("build");
        fs::create_dir_all(&build).expect("build dir");
        fs::write(build.join("demo.exe"), "binary").expect("artifact");

        let summary = crate::actions::project_summary_at(&temp);
        let state = super::load_project_state(&summary);

        assert!(state.manifest_exists);
        assert!(state.entry_exists);
        assert!(state.build_dir_exists);
        assert!(
            state
                .manifest_preview
                .iter()
                .any(|line| line.contains("[package]"))
        );
        assert!(
            state
                .build_artifacts
                .iter()
                .any(|line| line.contains("demo.exe"))
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn config_save_updates_manifest_on_disk() {
        let temp = unique_temp_dir("config_save");
        init_project(&temp).expect("project should init");

        let mut app = App::new();
        app.open_project_path(temp.clone());
        app.config.manifest.as_mut().expect("manifest").version = "1.1.0".to_string();
        app.save_config();

        let loaded = crate::project::load_manifest_config_at(&temp).expect("manifest reload");
        assert_eq!(loaded.version, "1.1.0");
        assert_eq!(
            app.last_action.as_ref().map(|action| action.name.as_str()),
            Some("config save")
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn config_dirty_tracks_unsaved_manifest_changes() {
        let temp = unique_temp_dir("config_dirty");
        init_project(&temp).expect("project should init");

        let mut app = App::new();
        app.open_project_path(temp.clone());
        assert!(!app.manifest_is_dirty());
        app.config.manifest.as_mut().expect("manifest").version = "9.9.9".to_string();
        assert!(app.manifest_is_dirty());
        app.save_config();
        assert!(!app.manifest_is_dirty());

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn current_build_options_use_tui_session_preferences() {
        let mut app = App::new();
        app.build_prefs.target = "x86_64-unknown-linux-gnu".to_string();
        app.build_prefs.compiler = Some("clang".to_string());

        let options = app.current_build_options();
        assert_eq!(options.target, "x86_64-unknown-linux-gnu");
        assert_eq!(options.cc.as_deref(), Some("clang"));
    }

    #[test]
    fn config_generate_entry_creates_missing_file() {
        let temp = unique_temp_dir("config_generate_entry");
        init_project(&temp).expect("project should init");

        let mut app = App::new();
        app.open_project_path(temp.clone());
        app.config.manifest.as_mut().expect("manifest").entry = "src/alt/main.skd".to_string();
        app.save_config();
        app.generate_entry_file();

        assert!(temp.join("src/alt/main.skd").exists());
        assert_eq!(
            app.last_action.as_ref().map(|action| action.name.as_str()),
            Some("entry generate")
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn test_backend_can_render_home_screen() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut app = App::new();
        terminal
            .draw(|frame| super::render(frame, &mut app))
            .expect("render should succeed");
    }
}
