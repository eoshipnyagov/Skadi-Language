use std::path::Path;
use std::process::Command;

use v01::codegen::IntWidth;

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

fn gnu_link_args(c: &str, out: &str, pthread: bool, windows: bool) -> Vec<String> {
    let mut args = vec![c.to_string(), "-o".to_string(), out.to_string()];
    if pthread {
        args.push("-pthread".to_string());
    }
    args.push("-lm".to_string());
    if windows {
        args.push("-lgdi32".to_string());
        args.push("-lws2_32".to_string());
    }
    args
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

pub fn resolve_int_width(target: &str, configured: &str) -> Result<IntWidth, String> {
    match configured.trim() {
        "target" => match target {
            "host" | "x86_64-w64-mingw32" | "x86_64-unknown-linux-gnu" => Ok(IntWidth::I32),
            other => Err(format!(
                "target '{other}' does not define a default Int width yet"
            )),
        },
        "i8" => Ok(IntWidth::I8),
        "i16" => Ok(IntWidth::I16),
        "i32" => Ok(IntWidth::I32),
        "i64" => Ok(IntWidth::I64),
        other => Err(format!(
            "unsupported numeric.int '{other}'; expected target, i8, i16, i32, or i64"
        )),
    }
}

pub fn candidate_invocations(
    target: &str,
    c_path: &Path,
    exe_path: &Path,
) -> Result<Vec<CompilerInvocation>, String> {
    let c = c_path.display().to_string();
    let out = exe_path.display().to_string();
    let object = c_path.with_extension("obj").display().to_string();
    let inv = match target {
        "host" => {
            let mut xs = vec![
                CompilerInvocation {
                    program: "gcc".to_string(),
                    args: gnu_link_args(&c, &out, !cfg!(windows), cfg!(windows)),
                },
                CompilerInvocation {
                    program: "clang".to_string(),
                    args: gnu_link_args(&c, &out, !cfg!(windows), cfg!(windows)),
                },
                CompilerInvocation {
                    program: "cc".to_string(),
                    args: gnu_link_args(&c, &out, !cfg!(windows), cfg!(windows)),
                },
            ];
            if cfg!(windows) {
                xs.push(CompilerInvocation {
                    program: "cl".to_string(),
                    args: vec![
                        "/nologo".to_string(),
                        c.clone(),
                        "gdi32.lib".to_string(),
                        "ws2_32.lib".to_string(),
                        format!("/Fo:{object}"),
                        format!("/Fe:{out}"),
                    ],
                });
            }
            xs
        }
        "x86_64-w64-mingw32" => vec![
            CompilerInvocation {
                program: "x86_64-w64-mingw32-gcc".to_string(),
                args: gnu_link_args(&c, &out, false, true),
            },
            CompilerInvocation {
                program: "gcc".to_string(),
                args: gnu_link_args(&c, &out, false, true),
            },
        ],
        "x86_64-unknown-linux-gnu" => vec![
            CompilerInvocation {
                program: "x86_64-linux-gnu-gcc".to_string(),
                args: gnu_link_args(&c, &out, true, false),
            },
            CompilerInvocation {
                program: "clang".to_string(),
                args: vec![
                    "--target=x86_64-unknown-linux-gnu".to_string(),
                    c.clone(),
                    "-o".to_string(),
                    out.clone(),
                    "-pthread".to_string(),
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
    let object = c_path.with_extension("obj").display().to_string();
    let inv = match compiler {
        "cl" => {
            if target != "host" {
                return Err(format!(
                    "compiler 'cl' is only supported for host target, got '{target}'"
                ));
            }
            CompilerInvocation {
                program: "cl".to_string(),
                args: vec![
                    "/nologo".to_string(),
                    c,
                    "gdi32.lib".to_string(),
                    "ws2_32.lib".to_string(),
                    format!("/Fo:{object}"),
                    format!("/Fe:{out}"),
                ],
            }
        }
        other => {
            let pthread =
                target == "x86_64-unknown-linux-gnu" || (target == "host" && !cfg!(windows));
            let windows = target == "x86_64-w64-mingw32" || (target == "host" && cfg!(windows));
            let mut args = gnu_link_args(&c, &out, pthread, windows);
            if target == "x86_64-unknown-linux-gnu" && other == "clang" {
                args.insert(0, "--target=x86_64-unknown-linux-gnu".to_string());
            }
            CompilerInvocation {
                program: other.to_string(),
                args,
            }
        }
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

fn linux_install_hint_with(mut available: impl FnMut(&str) -> bool) -> &'static str {
    if available("apt") || available("apt-get") {
        "Linux/WSL (apt): sudo apt install build-essential clang"
    } else if available("dnf") {
        "Linux (dnf): sudo dnf group install \"Development Tools\""
    } else if available("pacman") {
        "Linux (pacman): sudo pacman -S base-devel clang"
    } else if available("zypper") {
        "Linux (zypper): sudo zypper install -t pattern devel_basis"
    } else if available("apk") {
        "Linux (apk): sudo apk add build-base clang"
    } else {
        "Linux: install GCC or Clang with your system package manager."
    }
}

pub fn os_install_hint() -> String {
    if cfg!(windows) {
        "Windows: install MinGW-w64 (gcc) or Visual Studio Build Tools (cl).".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS: install Xcode Command Line Tools: xcode-select --install".to_string()
    } else {
        linux_install_hint_with(detect_compiler).to_string()
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

    use super::{candidate_invocations, linux_install_hint_with, single_compiler_invocation};

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
        assert!(inv.args.iter().any(|a| a == "-pthread"));
    }

    #[test]
    fn linux_profile_adds_pthread_link_flag() {
        let xs = candidate_invocations(
            "x86_64-unknown-linux-gnu",
            Path::new("a.c"),
            Path::new("a.out"),
        )
        .expect("linux invocations should be available");
        assert!(
            xs.iter()
                .all(|invocation| invocation.args.iter().any(|arg| arg == "-pthread"))
        );
    }

    #[test]
    fn windows_profile_links_canvas_backend_library() {
        let xs = candidate_invocations("x86_64-w64-mingw32", Path::new("a.c"), Path::new("a.exe"))
            .expect("Windows invocations should be available");
        assert!(
            xs.iter()
                .all(|invocation| invocation.args.iter().any(|arg| arg == "-lgdi32"))
        );
        assert!(
            xs.iter()
                .all(|invocation| invocation.args.iter().any(|arg| arg == "-lws2_32"))
        );
    }

    #[test]
    fn linux_install_hint_follows_detected_package_manager() {
        let hint = linux_install_hint_with(|program| program == "pacman");
        assert!(hint.contains("pacman -S base-devel"));

        let fallback = linux_install_hint_with(|_| false);
        assert!(fallback.contains("system package manager"));
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

    #[test]
    fn host_cl_places_object_beside_staged_c_source() {
        let invocation = single_compiler_invocation(
            "host",
            "cl",
            Path::new("temp/script.c"),
            Path::new("temp/script.exe"),
        )
        .expect("host cl invocation should be created");
        assert!(
            invocation
                .args
                .iter()
                .any(|arg| arg == "/Fo:temp/script.obj" || arg == "/Fo:temp\\script.obj")
        );
    }
}
