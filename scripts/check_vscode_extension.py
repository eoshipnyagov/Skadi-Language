from __future__ import annotations

import json
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parent.parent
EXTENSION = ROOT / "tools" / "vscode-skadi-syntax"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict:
    return json.loads(read(path))


def collect_regexes(value: object, output: list[tuple[str, str]], path: str = "") -> None:
    if isinstance(value, list):
        for index, item in enumerate(value):
            collect_regexes(item, output, f"{path}[{index}]")
        return
    if not isinstance(value, dict):
        return
    for key, item in value.items():
        child_path = f"{path}.{key}" if path else key
        if key in {"match", "begin", "end"} and isinstance(item, str):
            output.append((child_path, item))
        else:
            collect_regexes(item, output, child_path)


def alternatives(pattern: str, group: int) -> set[str]:
    groups = re.findall(r"\(([^()]+)\)", pattern)
    if group >= len(groups):
        return set()
    return set(groups[group].split("|"))


def main() -> int:
    errors: list[str] = []
    package_path = EXTENSION / "package.json"
    grammar_path = EXTENSION / "syntaxes" / "skadi.tmLanguage.json"
    snippets_path = EXTENSION / "snippets" / "skadi.json"

    package = load_json(package_path)
    grammar = load_json(grammar_path)
    load_json(EXTENSION / "language-configuration.json")
    load_json(snippets_path)

    version = package.get("version")
    readme = read(EXTENSION / "README.md")
    if not isinstance(version, str) or f"skadi-syntax-{version}.vsix" not in readme:
        errors.append("extension README install command does not match package version")

    snippet_entries = package.get("contributes", {}).get("snippets", [])
    expected_snippet_path = "./snippets/skadi.json"
    if not any(entry.get("path") == expected_snippet_path for entry in snippet_entries):
        errors.append("package.json does not contribute snippets/skadi.json")

    regexes: list[tuple[str, str]] = []
    collect_regexes(grammar, regexes)
    for path, pattern in regexes:
        try:
            re.compile(pattern)
        except re.error as error:
            errors.append(f"invalid grammar regex at {path}: {error}")

    builtin_patterns = grammar.get("repository", {}).get("builtins", {}).get("patterns", [])
    if len(builtin_patterns) < 2:
        errors.append("grammar does not define core and fs builtin patterns")
        highlighted_builtins: set[str] = set()
    else:
        highlighted_builtins = alternatives(builtin_patterns[0].get("match", ""), 0)
        highlighted_builtins.update(
            f"fs.{name}" for name in alternatives(builtin_patterns[1].get("match", ""), 1)
        )

    compiler_builtins = set(
        re.findall(r'name:\s*"([a-z_][a-z0-9_.]*)"', read(ROOT / "src" / "builtins.rs"))
    )
    missing_builtins = sorted(compiler_builtins - highlighted_builtins)
    extra_builtins = sorted(highlighted_builtins - compiler_builtins)
    if missing_builtins:
        errors.append(f"builtins missing from grammar: {', '.join(missing_builtins)}")
    if extra_builtins:
        errors.append(f"non-compiler builtins in grammar: {', '.join(extra_builtins)}")

    searchable_grammar = "\n".join(pattern.replace(r"\b", " ") for _, pattern in regexes)
    lexer_keywords = set(
        re.findall(
            r'"([a-z_]+)"\s*=>\s*TokenKind::Keyword[A-Za-z]+',
            read(ROOT / "src" / "lexer" / "core.rs"),
        )
    )
    missing_keywords = sorted(
        keyword
        for keyword in lexer_keywords
        if re.search(
            rf"(?<![A-Za-z_]){re.escape(keyword)}(?![A-Za-z_])",
            searchable_grammar,
        )
        is None
    )
    if missing_keywords:
        errors.append(f"lexer keywords missing from grammar: {', '.join(missing_keywords)}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(
        "VS Code extension check passed: "
        f"{len(regexes)} regexes, {len(compiler_builtins)} builtins, "
        f"{len(lexer_keywords)} lexer keywords."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
