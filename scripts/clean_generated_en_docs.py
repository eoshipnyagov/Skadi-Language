from __future__ import annotations

from pathlib import Path
import shutil
import stat


ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / ".docs-build"
SITE = ROOT / "site"


def remove_path(path: Path) -> None:
    if path.is_dir():
        shutil.rmtree(path, onerror=handle_remove_readonly)
        print(f"removed dir: {path}")
    elif path.exists():
        try:
            path.chmod(stat.S_IWRITE)
        except OSError:
            pass
        path.unlink()
        print(f"removed file: {path}")


def handle_remove_readonly(func, path, exc_info) -> None:
    Path(path).chmod(stat.S_IWRITE)
    func(path)


def main() -> None:
    if BUILD.exists():
        for path in BUILD.rglob("*.en.md"):
            remove_path(path)

    remove_path(SITE / "en")


if __name__ == "__main__":
    main()
