use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct CompilerInvocation {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub triple: &'static str,
    pub description: &'static str,
    pub output_kind: OutputKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    WindowsExe,
    LinuxElf,
}

const HOST_OUTPUT_KIND: OutputKind = if cfg!(windows) {
    OutputKind::WindowsExe
} else {
    OutputKind::LinuxElf
};

const PROFILES: [TargetProfile; 3] = [
    TargetProfile {
        triple: "host",
        description: "Current host toolchain auto-detection",
        output_kind: HOST_OUTPUT_KIND,
    },
    TargetProfile {
        triple: "x86_64-w64-mingw32",
        description: "Windows via MinGW GCC",
        output_kind: OutputKind::WindowsExe,
    },
    TargetProfile {
        triple: "x86_64-unknown-linux-gnu",
        description: "Linux GNU via cross GCC/Clang",
        output_kind: OutputKind::LinuxElf,
    },
];

pub fn builtin_profiles() -> &'static [TargetProfile] {
    &PROFILES
}

pub fn resolve_profile(target: &str) -> Result<TargetProfile, String> {
    builtin_profiles()
        .iter()
        .find(|p| p.triple == target)
        .cloned()
        .ok_or_else(|| format!("unknown target '{target}'. Use: skadi target list"))
}

pub fn candidate_invocations(
    target: &str,
    c_path: &Path,
    exe_path: &Path,
) -> Result<Vec<CompilerInvocation>, String> {
    let c = c_path.display().to_string();
    let out = exe_path.display().to_string();
    let inv = match target {
        "host" => {
            let mut xs = vec![
                CompilerInvocation {
                    program: "gcc".to_string(),
                    args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
                },
                CompilerInvocation {
                    program: "clang".to_string(),
                    args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
                },
                CompilerInvocation {
                    program: "cc".to_string(),
                    args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
                },
            ];
            if cfg!(windows) {
                xs.push(CompilerInvocation {
                    program: "cl".to_string(),
                    args: vec!["/nologo".to_string(), c.clone(), format!("/Fe:{out}")],
                });
            }
            xs
        }
        "x86_64-w64-mingw32" => vec![
            CompilerInvocation {
                program: "x86_64-w64-mingw32-gcc".to_string(),
                args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
            },
            CompilerInvocation {
                program: "gcc".to_string(),
                args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
            },
        ],
        "x86_64-unknown-linux-gnu" => vec![
            CompilerInvocation {
                program: "x86_64-linux-gnu-gcc".to_string(),
                args: vec![c.clone(), "-o".to_string(), out.clone(), "-lm".to_string()],
            },
            CompilerInvocation {
                program: "clang".to_string(),
                args: vec![
                    "--target=x86_64-unknown-linux-gnu".to_string(),
                    c.clone(),
                    "-o".to_string(),
                    out.clone(),
                    "-lm".to_string(),
                ],
            },
        ],
        other => return Err(format!("target '{other}' is not implemented yet.")),
    };
    Ok(inv)
}

pub fn single_compiler_invocation(
    target: &str,
    compiler: &str,
    c_path: &Path,
    exe_path: &Path,
) -> Result<CompilerInvocation, String> {
    let c = c_path.display().to_string();
    let out = exe_path.display().to_string();
    let inv = match compiler {
        "cl" => {
            if target != "host" {
                return Err(format!(
                    "compiler 'cl' is only supported for host target, got '{target}'"
                ));
            }
            CompilerInvocation {
                program: "cl".to_string(),
                args: vec!["/nologo".to_string(), c, format!("/Fe:{out}")],
            }
        }
        other => CompilerInvocation {
            program: other.to_string(),
            args: match target {
                "x86_64-unknown-linux-gnu" if other == "clang" => vec![
                    "--target=x86_64-unknown-linux-gnu".to_string(),
                    c,
                    "-o".to_string(),
                    out,
                    "-lm".to_string(),
                ],
                _ => vec![c, "-o".to_string(), out, "-lm".to_string()],
            },
        },
    };
    Ok(inv)
}

pub fn detect_compiler(program: &str) -> bool {
    Command::new(program).arg("--version").output().is_ok()
}

pub fn shell_probe_hint() -> &'static str {
    if cfg!(windows) {
        "where <compiler>"
    } else {
        "which <compiler>"
    }
}

pub fn os_install_hint() -> &'static str {
    if cfg!(windows) {
        "Windows: install MinGW-w64 (gcc) or Visual Studio Build Tools (cl)."
    } else if cfg!(target_os = "macos") {
        "macOS: install Xcode Command Line Tools: xcode-select --install"
    } else {
        "Linux/WSL: install build-essential (gcc) or clang (for example: sudo apt install build-essential clang)."
    }
}

pub fn target_hint(triple: &str) -> &'static str {
    match triple {
        "host" => "Install at least one host C compiler (gcc/clang/cc or cl on Windows).",
        "x86_64-w64-mingw32" => {
            "Install MinGW-w64 cross compiler (x86_64-w64-mingw32-gcc) or provide compatible gcc in PATH."
        }
        "x86_64-unknown-linux-gnu" => {
            "Install x86_64-linux-gnu-gcc cross toolchain or clang with Linux target support."
        }
        _ => "Install matching target toolchain and ensure compiler is available in PATH.",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{candidate_invocations, single_compiler_invocation};

    #[test]
    fn host_order_starts_with_gcc_clang() {
        let xs = candidate_invocations("host", Path::new("a.c"), Path::new("a.out"))
            .expect("host invocations should be available");
        assert!(xs.len() >= 2);
        assert_eq!(xs[0].program, "gcc");
        assert_eq!(xs[1].program, "clang");
    }

    #[test]
    fn explicit_clang_linux_cross_adds_target_flag() {
        let inv = single_compiler_invocation(
            "x86_64-unknown-linux-gnu",
            "clang",
            Path::new("a.c"),
            Path::new("a.out"),
        )
        .expect("explicit invocation should be created");
        assert!(
            inv.args
                .iter()
                .any(|a| a == "--target=x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn explicit_cl_rejected_for_non_host() {
        let err = single_compiler_invocation(
            "x86_64-unknown-linux-gnu",
            "cl",
            Path::new("a.c"),
            Path::new("a.out"),
        )
        .expect_err("cl should be host-only");
        assert!(err.contains("only supported for host"));
    }
}
