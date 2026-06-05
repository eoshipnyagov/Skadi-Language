use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use v01::formatter::format_source;

use crate::pipeline::{compile_c_to_exe_detailed, compile_frontend};
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
    pub target: String,
    pub requested_compiler: Option<String>,
    pub selected_compiler: String,
    pub compiler_args: Vec<String>,
    pub toolchain_status: String,
    pub toolchain_stdout: String,
    pub toolchain_stderr: String,
    pub c_path: PathBuf,
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
        entry: project.entry,
    })
}

pub fn run_build(options: &BuildOptions) -> Result<BuildResult, ActionError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ActionError::new(FailureSource::Io, format!("cwd failed: {e}")))?;
    run_build_at(&cwd, options)
}

pub fn run_build_at(root: &Path, options: &BuildOptions) -> Result<BuildResult, ActionError> {
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
    let frontend = compile_frontend(&project.entry).map_err(|e| {
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

    let exe_name = match profile.output_kind {
        OutputKind::WindowsExe => format!("{}.exe", project.name),
        OutputKind::LinuxElf => project.name.clone(),
    };
    let exe_path = build_dir.join(exe_name);

    let toolchain = match compile_c_to_exe_detailed(
        &c_path,
        &exe_path,
        &options.target,
        options.cc.as_deref(),
    ) {
        Ok(ok) => ok,
        Err(e) => {
            let compiler_info = options
                .cc
                .as_ref()
                .map(|v| format!("requested compiler: {v}"))
                .unwrap_or_else(|| {
                    "auto-detect order: gcc -> clang -> cc (and cl on Windows host)".to_string()
                });
            return Err(ActionError::new(
                FailureSource::Toolchain,
                format!(
                    "C toolchain error: {}\n{}\nhint: {}\ninstall: {}\nprobe: try 'gcc --version', 'clang --version', and '{}'",
                    e,
                    compiler_info,
                    target_hint(profile.triple),
                    os_install_hint(),
                    shell_probe_hint(),
                ),
            ));
        }
    };

    Ok(BuildResult {
        project: summary,
        warnings: frontend
            .warnings
            .iter()
            .flat_map(|w| parse_warning(w))
            .collect(),
        target: options.target.clone(),
        requested_compiler: options.cc.clone(),
        selected_compiler: toolchain.invocation.program,
        compiler_args: toolchain.invocation.args,
        toolchain_status: toolchain.status,
        toolchain_stdout: toolchain.stdout,
        toolchain_stderr: toolchain.stderr,
        c_path,
        exe_path,
    })
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
        host_install_hint: os_install_hint().to_string(),
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

pub fn parse_diagnostics(input: &str) -> Vec<DiagnosticSummary> {
    let mut diagnostics = Vec::new();
    for line in input.lines() {
        if let Some(diag) = parse_structured_diagnostic(line) {
            diagnostics.push(diag);
        }
    }
    if diagnostics.is_empty() {
        if let Some(diag) = parse_structured_diagnostic(input.trim()) {
            diagnostics.push(diag);
        }
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
    let (stage, rest, is_warning) = if let Some(x) = trimmed.strip_prefix("Semantic error at ") {
        ("Semantic".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Parse error at ") {
        ("Parse".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Lex error at ") {
        ("Lex".to_string(), x, false)
    } else if let Some(x) = trimmed.strip_prefix("Style warning at ") {
        ("Style".to_string(), x, true)
    } else {
        return None;
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
    }
}
