use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

pub use v01::analysis::{AnalysisFact, AnalysisFactLevel, AnalysisSubjectKind};
use v01::codegen::CodegenOptions;
use v01::formatter::format_source;

use crate::pipeline::{
    DebugSourceMapEntry, compile_c_to_exe_detailed, compile_frontend, compile_frontend_with_options,
};
use crate::project::{
    ManifestConfig, create_project, ensure_build_dir, ensure_entry_file_at, init_project,
    load_manifest_config_at, load_project_at, save_manifest_config_at,
};
use crate::targets::{
    OutputKind, builtin_profiles, candidate_invocations, detect_compiler, os_install_hint,
    resolve_profile, shell_probe_hint, target_hint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureSource {
    Frontend,
    Toolchain,
    Runtime,
    Project,
    Io,
    Usage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticSummary {
    pub stage: String,
    pub code: Option<String>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub message: String,
    pub is_warning: bool,
}

#[derive(Clone, Debug)]
pub struct ActionError {
    pub source: FailureSource,
    pub message: String,
    pub diagnostics: Vec<DiagnosticSummary>,
}

impl ActionError {
    pub fn new(source: FailureSource, message: impl Into<String>) -> Self {
        let message = message.into();
        let diagnostics = parse_diagnostics(&message);
        Self {
            source,
            message,
            diagnostics,
        }
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct ProjectSummary {
    pub cwd: PathBuf,
    pub manifest: PathBuf,
    pub build_dir: PathBuf,
    pub loaded: bool,
    pub name: Option<String>,
    pub entry: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub project: ProjectSummary,
    pub warnings: Vec<DiagnosticSummary>,
    pub analysis: Vec<AnalysisFact>,
    pub entry: PathBuf,
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub target: String,
    pub cc: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    pub project: ProjectSummary,
    pub warnings: Vec<DiagnosticSummary>,
    pub analysis: Vec<AnalysisFact>,
    pub target: String,
    pub requested_compiler: Option<String>,
    pub selected_compiler: String,
    pub compiler_args: Vec<String>,
    pub toolchain_status: String,
    pub toolchain_stdout: String,
    pub toolchain_stderr: String,
    pub c_path: PathBuf,
    pub debug_map_path: PathBuf,
    pub debug_map: Vec<DebugSourceMapEntry>,
    pub exe_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct RunResult {
    pub build: BuildResult,
    pub exit_status: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug)]
pub struct QuickRunOptions {
    pub source: PathBuf,
    pub build: BuildOptions,
    pub program_args: Vec<String>,
}

#[derive(Debug)]
pub struct QuickRunPrepared {
    pub source: PathBuf,
    pub target: String,
    pub selected_compiler: String,
    pub program_args: Vec<String>,
    exe_path: PathBuf,
    _build_dir: TemporaryBuildDir,
}

#[derive(Clone, Debug)]
pub struct QuickRunResult {
    pub source: PathBuf,
    pub target: String,
    pub selected_compiler: String,
    pub exit_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBreakpoint {
    pub path: PathBuf,
    pub line: u32,
}

#[derive(Clone, Debug)]
pub struct DebugOptions {
    pub build: BuildOptions,
    pub breakpoints: Vec<SourceBreakpoint>,
    pub program_args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBreakpoint {
    pub source_path: PathBuf,
    pub line: u32,
    pub col: u32,
    pub statement_id: String,
    pub statement_kind: &'static str,
}

#[derive(Clone, Debug)]
pub struct DebugPrepared {
    pub build: BuildResult,
    pub breakpoints: Vec<ResolvedBreakpoint>,
    pub starts_paused: bool,
    pub program_args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DebugResult {
    pub exit_status: String,
}

#[derive(Debug)]
struct TemporaryBuildDir {
    path: PathBuf,
}

impl Drop for TemporaryBuildDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone, Debug)]
pub struct BootstrapResult {
    pub root: PathBuf,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FormatOptions {
    pub check_only: bool,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatState {
    Updated,
    Unchanged,
}

#[derive(Clone, Debug)]
pub struct FormatFileResult {
    pub path: PathBuf,
    pub state: FormatState,
}

#[derive(Clone, Debug)]
pub struct FormatResult {
    pub check_only: bool,
    pub files: Vec<FormatFileResult>,
}

#[derive(Clone, Debug)]
pub struct TargetInfo {
    pub triple: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct TargetListResult {
    pub targets: Vec<TargetInfo>,
}

#[derive(Clone, Debug)]
pub struct CompilerStatus {
    pub program: String,
    pub available: bool,
}

#[derive(Clone, Debug)]
pub struct TargetStatus {
    pub triple: String,
    pub statuses: Vec<CompilerStatus>,
    pub ready: bool,
    pub hint: String,
}

#[derive(Clone, Debug)]
pub struct DoctorReport {
    pub host_candidates: Vec<CompilerStatus>,
    pub host_ready: bool,
    pub host_install_hint: String,
    pub shell_probe_hint: String,
    pub targets: Vec<TargetStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestConfigResult {
    pub manifest_path: PathBuf,
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
}

pub fn project_summary() -> ProjectSummary {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    project_summary_at(&cwd)
}

pub fn project_summary_at(root: &Path) -> ProjectSummary {
    let manifest = root.join("Skadi.toml");
    let build_dir = root.join("build");

    match load_project_at(root) {
        Ok(project) => ProjectSummary {
            cwd: root.to_path_buf(),
            manifest,
            build_dir,
            loaded: true,
            name: Some(project.name),
            entry: Some(project.entry),
        },
        Err(_) => ProjectSummary {
            cwd: root.to_path_buf(),
            manifest,
            build_dir,
            loaded: false,
            name: None,
            entry: None,
        },
    }
}

pub fn create_new_project(name: &str) -> Result<BootstrapResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    create_new_project_at(&cwd, name)
}

pub fn create_new_project_at(root: &Path, name: &str) -> Result<BootstrapResult, ActionError> {
    if name.trim().is_empty() {
        return Err(ActionError::new(
            FailureSource::Usage,
            "project name cannot be empty",
        ));
    }

    let project_root = root.join(name);
    if project_root.exists() {
        return Err(ActionError::new(
            FailureSource::Project,
            format!("Directory already exists: {}", project_root.display()),
        ));
    }
    let project_name = project_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ActionError::new(FailureSource::Usage, "invalid project path/name"))?
        .to_string();
    create_project(&project_root, &project_name)
        .map_err(|e| classify_fs_error(e, FailureSource::Io))?;
    Ok(BootstrapResult {
        root: project_root,
        name: Some(project_name),
    })
}

pub fn init_current_project() -> Result<BootstrapResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    init_project_at(&cwd)
}

pub fn init_project_at(root: &Path) -> Result<BootstrapResult, ActionError> {
    init_project(root).map_err(|e| classify_fs_error(e, FailureSource::Io))?;
    let project_name = load_project_at(root).ok().map(|p| p.name);
    Ok(BootstrapResult {
        root: root.to_path_buf(),
        name: project_name,
    })
}

pub fn parse_build_options(args: &[String]) -> Result<BuildOptions, ActionError> {
    let mut target = "host".to_string();
    let mut cc: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                if i + 1 >= args.len() {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--target requires value",
                    ));
                }
                target = args[i + 1].clone();
                i += 2;
            }
            "--cc" => {
                if i + 1 >= args.len() {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--cc requires value",
                    ));
                }
                cc = Some(args[i + 1].clone());
                i += 2;
            }
            other if other.starts_with("--") => {
                return Err(ActionError::new(
                    FailureSource::Usage,
                    format!("unknown build option: {other}"),
                ));
            }
            _ => i += 1,
        }
    }
    Ok(BuildOptions { target, cc })
}

pub fn parse_debug_options(args: &[String]) -> Result<DebugOptions, ActionError> {
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(args.len());
    let command_args = &args[..split];
    let program_args = if split < args.len() {
        args[(split + 1)..].to_vec()
    } else {
        Vec::new()
    };
    let mut target = "host".to_string();
    let mut cc = None;
    let mut breakpoints = Vec::new();
    let mut index = 0usize;

    while index < command_args.len() {
        match command_args[index].as_str() {
            "--target" => {
                let Some(value) = command_args.get(index + 1) else {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--target requires value",
                    ));
                };
                target = value.clone();
                index += 2;
            }
            "--cc" => {
                let Some(value) = command_args.get(index + 1) else {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--cc requires value",
                    ));
                };
                cc = Some(value.clone());
                index += 2;
            }
            "--break" | "-b" => {
                let Some(value) = command_args.get(index + 1) else {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--break requires <file.skd:line>",
                    ));
                };
                breakpoints.push(parse_source_breakpoint(value)?);
                index += 2;
            }
            other => {
                return Err(ActionError::new(
                    FailureSource::Usage,
                    format!("unknown debug option: {other}"),
                ));
            }
        }
    }

    Ok(DebugOptions {
        build: BuildOptions { target, cc },
        breakpoints,
        program_args,
    })
}

fn parse_source_breakpoint(value: &str) -> Result<SourceBreakpoint, ActionError> {
    let (path, line) = value.rsplit_once(':').ok_or_else(|| {
        ActionError::new(
            FailureSource::Usage,
            format!("invalid breakpoint '{value}'; expected <file.skd:line>"),
        )
    })?;
    if path.trim().is_empty() {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!("invalid breakpoint '{value}'; source path is empty"),
        ));
    }
    let line = line.parse::<u32>().map_err(|_| {
        ActionError::new(
            FailureSource::Usage,
            format!("invalid breakpoint '{value}'; line must be a positive integer"),
        )
    })?;
    if line == 0 {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!("invalid breakpoint '{value}'; line must be greater than zero"),
        ));
    }
    Ok(SourceBreakpoint {
        path: PathBuf::from(path),
        line,
    })
}

pub fn parse_quick_run_options(args: &[String]) -> Result<QuickRunOptions, ActionError> {
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(args.len());
    let command_args = &args[..split];
    let program_args = if split < args.len() {
        args[(split + 1)..].to_vec()
    } else {
        Vec::new()
    };

    let mut source = None;
    let mut target = "host".to_string();
    let mut cc = None;
    let mut index = 0usize;
    while index < command_args.len() {
        match command_args[index].as_str() {
            "--target" => {
                let Some(value) = command_args.get(index + 1) else {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--target requires value",
                    ));
                };
                target = value.clone();
                index += 2;
            }
            "--cc" => {
                let Some(value) = command_args.get(index + 1) else {
                    return Err(ActionError::new(
                        FailureSource::Usage,
                        "--cc requires value",
                    ));
                };
                cc = Some(value.clone());
                index += 2;
            }
            flag if flag.starts_with('-') => {
                return Err(ActionError::new(
                    FailureSource::Usage,
                    format!("unknown quick-run option: {flag}"),
                ));
            }
            path if source.is_none() => {
                source = Some(PathBuf::from(path));
                index += 1;
            }
            extra => {
                return Err(ActionError::new(
                    FailureSource::Usage,
                    format!(
                        "unexpected quick-run argument '{extra}'; pass program arguments after '--'"
                    ),
                ));
            }
        }
    }

    let source = source.ok_or_else(|| {
        ActionError::new(
            FailureSource::Usage,
            "Usage: skadi-cli quick-run <file.skd> [--cc compiler] [-- <args ...>]",
        )
    })?;
    Ok(QuickRunOptions {
        source,
        build: BuildOptions { target, cc },
        program_args,
    })
}

pub fn run_check() -> Result<CheckResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    run_check_at(&cwd)
}

pub fn run_check_at(root: &Path) -> Result<CheckResult, ActionError> {
    let project = load_project_at(root).map_err(|e| ActionError::new(FailureSource::Project, e))?;
    let summary = ProjectSummary {
        cwd: project.root.clone(),
        manifest: project.root.join("Skadi.toml"),
        build_dir: project.root.join("build"),
        loaded: true,
        name: Some(project.name),
        entry: Some(project.entry.clone()),
    };
    let frontend = compile_frontend(&project.entry).map_err(|e| {
        ActionError::new(
            FailureSource::Frontend,
            format!("Skadi frontend error: {e}"),
        )
    })?;
    Ok(CheckResult {
        project: summary,
        warnings: frontend
            .warnings
            .iter()
            .flat_map(|w| parse_warning(w))
            .collect(),
        analysis: frontend.analysis,
        entry: project.entry,
    })
}

pub fn run_build(options: &BuildOptions) -> Result<BuildResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    run_build_at(&cwd, options)
}

pub fn run_build_at(root: &Path, options: &BuildOptions) -> Result<BuildResult, ActionError> {
    run_build_at_mode(root, options, false)
}

fn run_build_at_mode(
    root: &Path,
    options: &BuildOptions,
    debug_probes: bool,
) -> Result<BuildResult, ActionError> {
    let profile =
        resolve_profile(&options.target).map_err(|e| ActionError::new(FailureSource::Usage, e))?;
    let project = load_project_at(root).map_err(|e| ActionError::new(FailureSource::Project, e))?;
    let summary = ProjectSummary {
        cwd: project.root.clone(),
        manifest: project.root.join("Skadi.toml"),
        build_dir: project.root.join("build"),
        loaded: true,
        name: Some(project.name.clone()),
        entry: Some(project.entry.clone()),
    };
    let frontend = compile_frontend_with_options(&project.entry, CodegenOptions { debug_probes })
        .map_err(|e| {
        ActionError::new(
            FailureSource::Frontend,
            format!("Skadi frontend error: {e}"),
        )
    })?;
    let build_dir =
        ensure_build_dir(&project.root).map_err(|e| ActionError::new(FailureSource::Io, e))?;
    let c_path = build_dir.join(format!("{}.c", project.name));
    fs::write(&c_path, frontend.c_code).map_err(|e| {
        ActionError::new(
            FailureSource::Io,
            format!(
                "build staging error: write {} failed: {e}",
                c_path.display()
            ),
        )
    })?;
    let debug_map = frontend.debug_map.clone();
    let debug_map_path = build_dir.join(format!("{}.skadi-debug.json", project.name));
    write_debug_map(
        &debug_map_path,
        &project.root,
        &project.entry,
        &c_path,
        &frontend.debug_map,
    )?;

    let exe_name = match profile.output_kind {
        OutputKind::WindowsExe => format!("{}.exe", project.name),
        OutputKind::LinuxElf => project.name.clone(),
    };
    let exe_path = build_dir.join(exe_name);

    let toolchain = compile_native(&c_path, &exe_path, options)?;

    Ok(BuildResult {
        project: summary,
        warnings: frontend
            .warnings
            .iter()
            .flat_map(|w| parse_warning(w))
            .collect(),
        analysis: frontend.analysis,
        target: options.target.clone(),
        requested_compiler: options.cc.clone(),
        selected_compiler: toolchain.invocation.program,
        compiler_args: toolchain.invocation.args,
        toolchain_status: toolchain.status,
        toolchain_stdout: toolchain.stdout,
        toolchain_stderr: toolchain.stderr,
        c_path,
        debug_map_path,
        debug_map,
        exe_path,
    })
}

fn write_debug_map(
    output_path: &Path,
    project_root: &Path,
    entry_path: &Path,
    c_path: &Path,
    entries: &[crate::pipeline::DebugSourceMapEntry],
) -> Result<(), ActionError> {
    let entry = portable_project_path(project_root, entry_path);
    let generated_c = portable_project_path(project_root, c_path);
    let entries = entries
        .iter()
        .map(|item| {
            json!({
                "statement_id": item.statement_id,
                "kind": item.statement_kind,
                "source": {
                    "path": portable_project_path(project_root, &item.source_path),
                    "line": item.source_line,
                    "col": item.source_col,
                },
                "generated": {
                    "start_line": item.generated_start_line,
                    "end_line": item.generated_end_line,
                },
            })
        })
        .collect::<Vec<_>>();
    let document = json!({
        "schema": "skadi.debug-map.v1",
        "entry": entry,
        "generated_c": generated_c,
        "entries": entries,
    });
    let encoded = serde_json::to_string_pretty(&document).map_err(|error| {
        ActionError::new(
            FailureSource::Io,
            format!("debug metadata serialization failed: {error}"),
        )
    })?;
    fs::write(output_path, encoded).map_err(|error| {
        ActionError::new(
            FailureSource::Io,
            format!(
                "build staging error: write {} failed: {error}",
                output_path.display()
            ),
        )
    })
}

fn portable_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn run_project(options: &BuildOptions) -> Result<RunResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    run_project_at(&cwd, options)
}

pub fn run_project_at(root: &Path, options: &BuildOptions) -> Result<RunResult, ActionError> {
    let build = run_build_at(root, options)?;
    let output = Command::new(&build.exe_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            ActionError::new(
                FailureSource::Runtime,
                format!(
                    "runtime execution error: failed to run {}: {e}",
                    build.exe_path.display()
                ),
            )
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let mut message = format!(
            "runtime execution error: program exited with status {}",
            output.status
        );
        if !stdout.trim().is_empty() {
            message.push_str("\nstdout:");
            for line in stdout.lines() {
                message.push_str("\n  ");
                message.push_str(line);
            }
        }
        if !stderr.trim().is_empty() {
            message.push_str("\nstderr:");
            for line in stderr.lines() {
                message.push_str("\n  ");
                message.push_str(line);
            }
        }
        return Err(ActionError::new(FailureSource::Runtime, message));
    }
    Ok(RunResult {
        build,
        exit_status: output.status.to_string(),
        stdout,
        stderr,
    })
}

pub fn prepare_debug(options: &DebugOptions) -> Result<DebugPrepared, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    prepare_debug_at(&cwd, options)
}

pub fn prepare_debug_at(root: &Path, options: &DebugOptions) -> Result<DebugPrepared, ActionError> {
    if options.build.target != "host" {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!(
                "debug executes host programs only, got target '{}'; use `skadi-cli build --target {}` for cross-target artifacts",
                options.build.target, options.build.target
            ),
        ));
    }
    let build = run_build_at_mode(root, &options.build, true)?;
    let mut breakpoints = Vec::new();
    let mut seen_statement_ids = BTreeSet::new();
    for requested in &options.breakpoints {
        for resolved in resolve_breakpoint(&build, requested)? {
            if seen_statement_ids.insert(resolved.statement_id.clone()) {
                breakpoints.push(resolved);
            }
        }
    }
    Ok(DebugPrepared {
        build,
        starts_paused: breakpoints.is_empty(),
        breakpoints,
        program_args: options.program_args.clone(),
    })
}

fn resolve_breakpoint(
    build: &BuildResult,
    requested: &SourceBreakpoint,
) -> Result<Vec<ResolvedBreakpoint>, ActionError> {
    let requested_path = if requested.path.is_absolute() {
        requested.path.clone()
    } else {
        build.project.cwd.join(&requested.path)
    };
    let requested_path = fs::canonicalize(&requested_path).map_err(|error| {
        ActionError::new(
            FailureSource::Usage,
            format!(
                "breakpoint source '{}' cannot be opened: {error}",
                requested_path.display()
            ),
        )
    })?;
    let source_entries = build
        .debug_map
        .iter()
        .filter(|entry| same_debug_path(&entry.source_path, &requested_path))
        .collect::<Vec<_>>();
    if source_entries.is_empty() {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!(
                "breakpoint source '{}' is not part of the current project import graph",
                requested.path.display()
            ),
        ));
    }
    let matches = source_entries
        .iter()
        .filter(|entry| entry.source_line == requested.line)
        .map(|entry| ResolvedBreakpoint {
            source_path: entry.source_path.clone(),
            line: entry.source_line,
            col: entry.source_col,
            statement_id: entry.statement_id.clone(),
            statement_kind: entry.statement_kind,
        })
        .collect::<Vec<_>>();
    if !matches.is_empty() {
        return Ok(matches);
    }

    let mut lines = source_entries
        .iter()
        .map(|entry| entry.source_line)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    lines.sort_by_key(|line| (line.abs_diff(requested.line), *line));
    let nearest = lines
        .into_iter()
        .take(5)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(ActionError::new(
        FailureSource::Usage,
        format!(
            "no executable statement at '{}:{}'; nearest executable lines: {}",
            requested.path.display(),
            requested.line,
            nearest
        ),
    ))
}

fn same_debug_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

pub fn execute_debug(prepared: &DebugPrepared) -> Result<DebugResult, ActionError> {
    let breakpoint_ids = prepared
        .breakpoints
        .iter()
        .map(|breakpoint| breakpoint.statement_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let status = Command::new(&prepared.build.exe_path)
        .current_dir(&prepared.build.project.cwd)
        .args(&prepared.program_args)
        .env("SKADI_DEBUG_BREAKPOINTS", breakpoint_ids)
        .env(
            "SKADI_DEBUG_STEP",
            if prepared.starts_paused { "1" } else { "0" },
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            ActionError::new(
                FailureSource::Runtime,
                format!(
                    "debug execution error: failed to run {}: {error}",
                    prepared.build.exe_path.display()
                ),
            )
        })?;
    if !status.success() {
        return Err(ActionError::new(
            FailureSource::Runtime,
            format!("debug execution error: program exited with status {status}"),
        ));
    }
    Ok(DebugResult {
        exit_status: status.to_string(),
    })
}

pub fn prepare_quick_run(options: &QuickRunOptions) -> Result<QuickRunPrepared, ActionError> {
    if options.build.target != "host" {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!(
                "quick-run executes host programs only, got target '{}'; use a project build for cross-target artifacts",
                options.build.target
            ),
        ));
    }

    let source = if options.source.is_absolute() {
        options.source.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?
            .join(&options.source)
    };
    if !source.exists() {
        return Err(ActionError::new(
            FailureSource::Io,
            format!("quick-run source '{}' does not exist", source.display()),
        ));
    }
    if !source.is_file() {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!("quick-run source '{}' is not a file", source.display()),
        ));
    }
    if !source
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("skd"))
    {
        return Err(ActionError::new(
            FailureSource::Usage,
            format!("quick-run expects a .skd file, got '{}'", source.display()),
        ));
    }

    let frontend = compile_frontend(&source).map_err(|e| {
        ActionError::new(
            FailureSource::Frontend,
            format!("Skadi frontend error: {e}"),
        )
    })?;
    let build_dir = create_temporary_build_dir()?;
    let artifact_name = quick_artifact_name(&source);
    let c_path = build_dir.path.join(format!("{artifact_name}.c"));
    fs::write(&c_path, frontend.c_code).map_err(|e| {
        ActionError::new(
            FailureSource::Io,
            format!(
                "quick-run staging error: write {} failed: {e}",
                c_path.display()
            ),
        )
    })?;

    let profile = resolve_profile(&options.build.target)
        .map_err(|e| ActionError::new(FailureSource::Usage, e))?;
    let exe_name = match profile.output_kind {
        OutputKind::WindowsExe => format!("{artifact_name}.exe"),
        OutputKind::LinuxElf => artifact_name,
    };
    let exe_path = build_dir.path.join(exe_name);
    let toolchain = compile_native(&c_path, &exe_path, &options.build)?;

    Ok(QuickRunPrepared {
        source,
        target: options.build.target.clone(),
        selected_compiler: toolchain.invocation.program,
        program_args: options.program_args.clone(),
        exe_path,
        _build_dir: build_dir,
    })
}

pub fn execute_quick_run(prepared: &QuickRunPrepared) -> Result<QuickRunResult, ActionError> {
    let status = Command::new(&prepared.exe_path)
        .args(&prepared.program_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| {
            ActionError::new(
                FailureSource::Runtime,
                format!(
                    "runtime execution error: failed to run {}: {e}",
                    prepared.source.display()
                ),
            )
        })?;
    if !status.success() {
        return Err(ActionError::new(
            FailureSource::Runtime,
            format!(
                "runtime execution error: '{}' exited with status {}",
                prepared.source.display(),
                status
            ),
        ));
    }
    Ok(QuickRunResult {
        source: prepared.source.clone(),
        target: prepared.target.clone(),
        selected_compiler: prepared.selected_compiler.clone(),
        exit_status: status.to_string(),
    })
}

pub fn parse_format_options(args: &[String]) -> Result<FormatOptions, ActionError> {
    let mut check_only = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--check" => check_only = true,
            "--write" => check_only = false,
            flag if flag.starts_with('-') => {
                return Err(ActionError::new(
                    FailureSource::Usage,
                    format!("unknown format option: {flag}"),
                ));
            }
            _ => paths.push(arg.clone()),
        }
    }

    Ok(FormatOptions { check_only, paths })
}

pub fn run_format(options: &FormatOptions) -> Result<FormatResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    run_format_at(&cwd, options)
}

pub fn run_format_at(root: &Path, options: &FormatOptions) -> Result<FormatResult, ActionError> {
    let paths = resolve_format_targets(root, &options.paths)?;
    let mut files = Vec::new();

    for path in paths {
        let state = if options.check_only {
            check_file(&path)?
        } else {
            format_file(&path)?
        };
        files.push(FormatFileResult { path, state });
    }

    Ok(FormatResult {
        check_only: options.check_only,
        files,
    })
}

pub fn list_targets() -> TargetListResult {
    TargetListResult {
        targets: builtin_profiles()
            .iter()
            .map(|p| TargetInfo {
                triple: p.triple.to_string(),
                description: p.description.to_string(),
            })
            .collect(),
    }
}

pub fn run_doctor() -> Result<DoctorReport, ActionError> {
    let host_dummy_c = Path::new("dummy.c");
    let host_dummy_out = Path::new("dummy.out");
    let host_candidates = candidate_invocations("host", host_dummy_c, host_dummy_out)
        .map_err(|e| ActionError::new(FailureSource::Toolchain, e))?;
    let mut host_seen: BTreeSet<String> = BTreeSet::new();
    let mut host_statuses = Vec::new();
    let mut host_ready = false;
    for candidate in host_candidates {
        if !host_seen.insert(candidate.program.clone()) {
            continue;
        }
        let available = detect_compiler(&candidate.program);
        host_ready |= available;
        host_statuses.push(CompilerStatus {
            program: candidate.program,
            available,
        });
    }

    let mut targets = Vec::new();
    for profile in builtin_profiles() {
        let candidates = candidate_invocations(profile.triple, host_dummy_c, host_dummy_out)
            .map_err(|e| ActionError::new(FailureSource::Toolchain, e))?;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut statuses = Vec::new();
        let mut ready = false;
        for candidate in candidates {
            if !seen.insert(candidate.program.clone()) {
                continue;
            }
            let available = detect_compiler(&candidate.program);
            ready |= available;
            statuses.push(CompilerStatus {
                program: candidate.program,
                available,
            });
        }
        targets.push(TargetStatus {
            triple: profile.triple.to_string(),
            statuses,
            ready,
            hint: target_hint(profile.triple).to_string(),
        });
    }

    Ok(DoctorReport {
        host_candidates: host_statuses,
        host_ready,
        host_install_hint: os_install_hint(),
        shell_probe_hint: shell_probe_hint().to_string(),
        targets,
    })
}

pub fn load_manifest_config(root: &Path) -> Result<ManifestConfigResult, ActionError> {
    let manifest =
        load_manifest_config_at(root).map_err(|e| ActionError::new(FailureSource::Project, e))?;
    Ok(ManifestConfigResult {
        manifest_path: root.join("Skadi.toml"),
        name: manifest.name,
        version: manifest.version,
        edition: manifest.edition,
        entry: manifest.entry,
    })
}

pub fn save_manifest_config(
    root: &Path,
    manifest: &ManifestConfigResult,
) -> Result<ManifestConfigResult, ActionError> {
    let updated = ManifestConfig {
        name: manifest.name.trim().to_string(),
        version: manifest.version.trim().to_string(),
        edition: manifest.edition.trim().to_string(),
        entry: manifest.entry.trim().to_string(),
    };
    save_manifest_config_at(root, &updated).map_err(|e| ActionError::new(FailureSource::Io, e))?;
    load_manifest_config(root)
}

pub fn ensure_project_entry_file(root: &Path) -> Result<PathBuf, ActionError> {
    let manifest =
        load_manifest_config_at(root).map_err(|e| ActionError::new(FailureSource::Project, e))?;
    ensure_entry_file_at(root, &manifest.entry).map_err(|e| ActionError::new(FailureSource::Io, e))
}

fn compile_native(
    c_path: &Path,
    exe_path: &Path,
    options: &BuildOptions,
) -> Result<crate::pipeline::ToolchainOutput, ActionError> {
    let profile =
        resolve_profile(&options.target).map_err(|e| ActionError::new(FailureSource::Usage, e))?;
    compile_c_to_exe_detailed(
        c_path,
        exe_path,
        &options.target,
        options.cc.as_deref(),
    )
    .map_err(|e| {
        let compiler_info = options
            .cc
            .as_ref()
            .map(|value| format!("requested compiler: {value}"))
            .unwrap_or_else(|| {
                "auto-detect order: gcc -> clang -> cc (and cl on Windows host)".to_string()
            });
        ActionError::new(
            FailureSource::Toolchain,
            format!(
                "C toolchain error: {}\n{}\nhint: {}\ninstall: {}\nprobe: try 'gcc --version', 'clang --version', and '{}'",
                e,
                compiler_info,
                target_hint(profile.triple),
                os_install_hint(),
                shell_probe_hint(),
            ),
        )
    })
}

fn create_temporary_build_dir() -> Result<TemporaryBuildDir, ActionError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ActionError::new(FailureSource::Io, format!("clock failed: {e}")))?
        .as_nanos();
    let parent = std::env::temp_dir();
    for attempt in 0..16u8 {
        let path = parent.join(format!(
            "skadi-quick-run-{}-{stamp}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(TemporaryBuildDir { path }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ActionError::new(
                    FailureSource::Io,
                    format!(
                        "quick-run temporary directory '{}' failed: {error}",
                        path.display()
                    ),
                ));
            }
        }
    }
    Err(ActionError::new(
        FailureSource::Io,
        "quick-run could not allocate a unique temporary build directory",
    ))
}

fn quick_artifact_name(source: &Path) -> String {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("script");
    let sanitized = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "script".to_string()
    } else {
        sanitized
    }
}

pub fn parse_diagnostics(input: &str) -> Vec<DiagnosticSummary> {
    let mut diagnostics = Vec::new();
    for line in input.lines() {
        if let Some(diag) = parse_structured_diagnostic(line) {
            diagnostics.push(diag);
        }
    }
    if diagnostics.is_empty()
        && let Some(diag) = parse_structured_diagnostic(input.trim())
    {
        diagnostics.push(diag);
    }
    diagnostics
}

fn parse_warning(input: &str) -> Vec<DiagnosticSummary> {
    let mut diagnostics = parse_diagnostics(input);
    for diag in &mut diagnostics {
        diag.is_warning = true;
    }
    diagnostics
}

fn parse_structured_diagnostic(input: &str) -> Option<DiagnosticSummary> {
    let trimmed = input.trim();
    let trimmed = [
        "Semantic error at ",
        "Semantic error:",
        "Parse error at ",
        "Parse error:",
        "Lex error at ",
        "Lex error:",
        "Style warning at ",
    ]
    .iter()
    .filter_map(|marker| trimmed.find(marker))
    .min()
    .map(|position| &trimmed[position..])
    .unwrap_or(trimmed);
    let (stage, rest, is_warning) = if let Some(x) = trimmed.strip_prefix("Semantic error at ") {
        ("Semantic".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Parse error at ") {
        ("Parse".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Lex error at ") {
        ("Lex".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Semantic error:") {
        ("Semantic".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Parse error:") {
        ("Parse".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Lex error:") {
        ("Lex".to_string(), x, false)
    } else {
        let x = trimmed.strip_prefix("Style warning at ")?;
        ("Style".to_string(), x, true)
    };

    let line = extract_after(rest, "line ").and_then(extract_leading_number);
    let col = extract_after(rest, "col ").and_then(extract_leading_number);
    let code = extract_between(rest, "[", "]");
    let message = if let Some(pos) = rest.find("] ") {
        rest[(pos + 2)..].trim().to_string()
    } else if let Some(pos) = rest.find(": ") {
        rest[(pos + 2)..].trim().to_string()
    } else {
        rest.trim().to_string()
    };

    Some(DiagnosticSummary {
        stage,
        code,
        line,
        col,
        message,
        is_warning,
    })
}

fn resolve_format_targets(root: &Path, args: &[String]) -> Result<Vec<PathBuf>, ActionError> {
    if args.is_empty() {
        let project =
            load_project_at(root).map_err(|e| ActionError::new(FailureSource::Project, e))?;
        return Ok(vec![project.entry]);
    }

    Ok(args.iter().map(|arg| root.join(arg)).collect())
}

fn format_file(path: &Path) -> Result<FormatState, ActionError> {
    let source = fs::read_to_string(path).map_err(|e| {
        ActionError::new(
            FailureSource::Io,
            format!("failed to read {}: {e}", path.display()),
        )
    })?;
    let formatted = format_source(&source).map_err(|e| {
        ActionError::new(
            FailureSource::Frontend,
            format!("failed to format {}: {e}", path.display()),
        )
    })?;

    if normalize_newlines(&source) == normalize_newlines(&formatted) {
        return Ok(FormatState::Unchanged);
    }

    fs::write(path, formatted).map_err(|e| {
        ActionError::new(
            FailureSource::Io,
            format!("failed to write {}: {e}", path.display()),
        )
    })?;
    Ok(FormatState::Updated)
}

fn check_file(path: &Path) -> Result<FormatState, ActionError> {
    let source = fs::read_to_string(path).map_err(|e| {
        ActionError::new(
            FailureSource::Io,
            format!("failed to read {}: {e}", path.display()),
        )
    })?;
    let formatted = format_source(&source).map_err(|e| {
        ActionError::new(
            FailureSource::Frontend,
            format!("failed to format {}: {e}", path.display()),
        )
    })?;

    if normalize_newlines(&source) == normalize_newlines(&formatted) {
        Ok(FormatState::Unchanged)
    } else {
        Ok(FormatState::Updated)
    }
}

fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n")
}

fn classify_fs_error(message: String, fallback: FailureSource) -> ActionError {
    if message.starts_with("write ") || message.starts_with("create ") {
        ActionError::new(FailureSource::Io, message)
    } else {
        ActionError::new(fallback, message)
    }
}

fn extract_after<'a>(s: &'a str, needle: &str) -> Option<&'a str> {
    let idx = s.find(needle)?;
    Some(&s[(idx + needle.len())..])
}

fn extract_between(s: &str, left: &str, right: &str) -> Option<String> {
    let start = s.find(left)? + left.len();
    let tail = &s[start..];
    let end = tail.find(right)?;
    Some(tail[..end].to_string())
}

fn extract_leading_number(s: &str) -> Option<u32> {
    let digits: String = s
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{FailureSource, parse_build_options, parse_diagnostics, parse_format_options};

    #[test]
    fn parse_build_options_reports_usage_errors() {
        let err = parse_build_options(&["--cc".to_string()]).expect_err("should fail");
        assert_eq!(err.source, FailureSource::Usage);
        assert!(err.to_string().contains("--cc requires value"));
    }

    #[test]
    fn parse_format_options_rejects_unknown_flag() {
        let err = parse_format_options(&["--wat".to_string()]).expect_err("should fail");
        assert_eq!(err.source, FailureSource::Usage);
        assert!(err.to_string().contains("unknown format option"));
    }

    #[test]
    fn parse_diagnostics_extracts_shape() {
        let xs =
            parse_diagnostics("Semantic error at line 2, col 5 [SC-SEM-020]: undefined symbol 'x'");
        assert_eq!(xs.len(), 1);
        assert_eq!(xs[0].stage, "Semantic");
        assert_eq!(xs[0].code.as_deref(), Some("SC-SEM-020"));
        assert_eq!(xs[0].line, Some(2));
        assert_eq!(xs[0].col, Some(5));

        let wrapped = parse_diagnostics(
            "Skadi frontend error: [SC-SEM-000] stage=semantic: Semantic error at line 3, col 7 [SC-SEM-020]: undefined symbol 'y'",
        );
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].code.as_deref(), Some("SC-SEM-020"));
        assert_eq!(wrapped[0].line, Some(3));
    }
}
