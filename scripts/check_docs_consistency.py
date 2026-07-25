from __future__ import annotations

from pathlib import Path
import re
import sys

from sync_docs_site import ROUTE_MAP, find_ru_source


ROOT = Path(__file__).resolve().parent.parent


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def skadi_blocks(text: str) -> list[str]:
    return re.findall(r"```(?:skadi|scadi)\s*\n(.*?)```", text, flags=re.S | re.I)


def main() -> int:
    errors: list[str] = []
    cargo_toml = read(ROOT / "Cargo.toml")
    version_match = re.search(
        r"(?ms)^\[workspace\.package\]\s+.*?^version\s*=\s*\"([^\"]+)\"",
        cargo_toml,
    )
    if version_match is None:
        errors.append("Cargo.toml does not define [workspace.package] version")
        release_version = ""
    else:
        release_version = version_match.group(1)

    markdown_files = [ROOT / "README.md"]
    markdown_files.extend((ROOT / "docs").glob("*.md"))
    markdown_files.extend((ROOT / "docs-en").glob("*.md"))
    markdown_files.append(ROOT / "tools" / "skadi-cli" / "README.md")

    forbidden = {
        "personal Windows path": re.compile(r"[A-Za-z]:[\\/].*(?:Yandex|Skadi[\\/]v01)", re.I),
        "personal v01 location": re.compile(r"Skadi[\\/]v01", re.I),
        "incorrect CLI manifest path": re.compile(
            r"cargo run --manifest-path skadi-cli/Cargo\.toml"
        ),
        "repository-specific working directory wording": re.compile(
            r"[Вв] рабочей директории репозитория"
        ),
        "obsolete pending remote matrix wording": re.compile(
            r"оста[её]тся experimental до успешной проверки dedicated TSan"
        ),
        "obsolete CLI product identity": re.compile(r"skadi-cli(?:\s+|`?\s*)v1\.1", re.I),
    }

    for path in markdown_files:
        text = read(path)
        for label, pattern in forbidden.items():
            match = pattern.search(text)
            if match is not None:
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{path.relative_to(ROOT)}:{line}: {label}")

    for route, source in ROUTE_MAP:
        source_path = find_ru_source(source)
        if not source_path.is_file():
            errors.append(f"missing RU source for route '{route}': {source}")

    required_user_routes = {"user/installation", "user/v1-2-migration"}
    actual_routes = {route for route, _ in ROUTE_MAP}
    for route in sorted(required_user_routes - actual_routes):
        errors.append(f"missing required release documentation route '{route}'")

    current_version_docs = [
        ROOT / "README.md",
        ROOT / "tools" / "skadi-cli" / "README.md",
        ROOT / "docs" / "SKADI_INSTALLATION_RU.md",
        ROOT / "docs-en" / "SKADI_INSTALLATION_EN.md",
        ROOT / "docs" / "SKADI_V1_2_MIGRATION_RU.md",
        ROOT / "docs-en" / "SKADI_V1_2_MIGRATION_EN.md",
    ]
    if release_version:
        for path in current_version_docs:
            if release_version not in read(path):
                errors.append(
                    f"{path.relative_to(ROOT)} does not mention current release "
                    f"version '{release_version}'"
                )

    required_release_files = [
        ROOT / "install" / "install.ps1",
        ROOT / "install" / "uninstall.ps1",
        ROOT / "install" / "install.sh",
        ROOT / "install" / "uninstall.sh",
        ROOT / ".github" / "workflows" / "release.yml",
    ]
    for path in required_release_files:
        if not path.is_file():
            errors.append(f"missing release file: {path.relative_to(ROOT)}")

    user_sources = {ROOT / "README.md"}
    user_sources.update(
        find_ru_source(source) for route, source in ROUTE_MAP if route.startswith("user/")
    )
    unsupported_skadi_patterns = {
        "legacy List(T) type syntax": re.compile(r"\bList\s*\("),
        "typed declaration combined with on error": re.compile(
            r"\bnew\b[^\n]*\bon error\b"
        ),
        "symbolic function return arrow": re.compile(r"\bfn\b[^\n{]*->"),
        "non-danger I/O call with on error": re.compile(
            r"\b(?:read|write|input|output|fs\.list|fs\.join|fs\.is_dir)\s*\([^\n]*\)\s+on error\b"
        ),
    }
    for path in sorted(user_sources):
        for block_index, block in enumerate(skadi_blocks(read(path)), start=1):
            for label, pattern in unsupported_skadi_patterns.items():
                if pattern.search(block):
                    errors.append(
                        f"{path.relative_to(ROOT)}: Skadi block {block_index}: {label}"
                    )

    language_reference = read(ROOT / "docs" / "SKADI_LANGUAGE_REFERENCE_RU.md")
    builtin_source = read(ROOT / "src" / "builtins.rs")
    builtin_names = sorted(set(re.findall(r'name:\s*"([^"]+)"', builtin_source)))
    for name in builtin_names:
        if f"`{name}" not in language_reference:
            errors.append(f"language reference does not mention builtin '{name}'")

    syntax_status = read(ROOT / "docs" / "SKADI_SYNTAX_STATUS.md")
    required_surfaces = [
        "Memory",
        "Task",
        "Channel",
        "Time",
        "Duration",
        "ByteSize",
        "Angle",
        "Vec2",
        "Vec3",
        "Vec4",
        "place in",
        "stopping",
        "iterate",
        "returns",
    ]
    for surface in required_surfaces:
        if surface not in syntax_status:
            errors.append(f"syntax status does not mention '{surface}'")

    cli_reference = read(ROOT / "docs" / "SKADI_CLI_REFERENCE_RU.md")
    for command in ["new", "init", "check", "build", "run", "target list", "format", "doctor", "tui"]:
        if f"`{command}" not in cli_reference:
            errors.append(f"CLI reference does not mention command '{command}'")

    if errors:
        print("Documentation consistency check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        f"Documentation consistency check passed: {len(markdown_files)} Markdown files, "
        f"{len(ROUTE_MAP)} routes, {len(builtin_names)} builtins."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
