from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
from pathlib import Path
import re
import tarfile
import zipfile


ROOT = Path(__file__).resolve().parents[2]
VERSION_PATTERN = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")


def normalize_version(raw: str) -> str:
    version = raw.removeprefix("v")
    if not VERSION_PATTERN.fullmatch(version):
        raise ValueError(f"invalid release version: {raw!r}")
    return version


def workspace_version() -> str:
    cargo_toml = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(
        r"(?ms)^\[workspace\.package\]\s+.*?^version\s*=\s*\"([^\"]+)\"",
        cargo_toml,
    )
    if match is None:
        raise ValueError("Cargo.toml does not define [workspace.package] version")
    return normalize_version(match.group(1))


def release_entries(binary: Path, version: str, target: str) -> list[tuple[str, bytes, int]]:
    if not binary.is_file():
        raise FileNotFoundError(f"release binary does not exist: {binary}")
    binary_name = "skadi-cli.exe" if "windows" in target else "skadi-cli"
    root_name = f"skadi-cli-v{version}-{target}"
    source_files = [
        ("LICENSE", ROOT / "LICENSE"),
        ("README.md", ROOT / "README.md"),
        ("CHANGELOG.md", ROOT / "CHANGELOG.md"),
    ]
    entries = [
        (f"{root_name}/{binary_name}", binary.read_bytes(), 0o755),
        (f"{root_name}/VERSION", f"{version}\n".encode(), 0o644),
    ]
    for archive_name, source in source_files:
        if not source.is_file():
            raise FileNotFoundError(f"required release file does not exist: {source}")
        entries.append((f"{root_name}/{archive_name}", source.read_bytes(), 0o644))
    return sorted(entries, key=lambda entry: entry[0])


def write_zip(path: Path, entries: list[tuple[str, bytes, int]]) -> None:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, payload, mode in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (mode & 0xFFFF) << 16
            archive.writestr(info, payload, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


def write_tar_gz(path: Path, entries: list[tuple[str, bytes, int]]) -> None:
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
    with path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch, compresslevel=9) as gz:
            with tarfile.open(mode="w", fileobj=gz, format=tarfile.PAX_FORMAT) as archive:
                for name, payload, mode in entries:
                    info = tarfile.TarInfo(name)
                    info.size = len(payload)
                    info.mode = mode
                    info.mtime = epoch
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    archive.addfile(info, io.BytesIO(payload))


def package(binary: Path, target: str, raw_version: str, output_dir: Path) -> Path:
    version = normalize_version(raw_version)
    expected = workspace_version()
    if version != expected:
        raise ValueError(
            f"requested version {version} does not match Cargo workspace version {expected}"
        )
    output_dir.mkdir(parents=True, exist_ok=True)
    suffix = ".zip" if "windows" in target else ".tar.gz"
    archive = output_dir / f"skadi-cli-v{version}-{target}{suffix}"
    entries = release_entries(binary, version, target)
    if suffix == ".zip":
        write_zip(archive, entries)
    else:
        write_tar_gz(archive, entries)
    return archive


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_checksums(dist_dir: Path, output: Path) -> Path:
    archives = sorted(
        path
        for path in dist_dir.iterdir()
        if path.is_file() and (path.suffix == ".zip" or path.name.endswith(".tar.gz"))
    )
    if not archives:
        raise ValueError(f"no release archives found in {dist_dir}")
    output.parent.mkdir(parents=True, exist_ok=True)
    content = "".join(f"{sha256(path)}  {path.name}\n" for path in archives)
    output.write_text(content, encoding="ascii", newline="\n")
    return output


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description="Build deterministic Skadi release archives")
    commands = root.add_subparsers(dest="command", required=True)

    verify_cmd = commands.add_parser("verify-version")
    verify_cmd.add_argument("--version", required=True)

    package_cmd = commands.add_parser("package")
    package_cmd.add_argument("--binary", type=Path, required=True)
    package_cmd.add_argument("--target", required=True)
    package_cmd.add_argument("--version", required=True)
    package_cmd.add_argument("--output-dir", type=Path, required=True)

    checksums_cmd = commands.add_parser("checksums")
    checksums_cmd.add_argument("--dist-dir", type=Path, required=True)
    checksums_cmd.add_argument("--output", type=Path, required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    if args.command == "verify-version":
        version = normalize_version(args.version)
        expected = workspace_version()
        if version != expected:
            raise ValueError(
                f"requested version {version} does not match Cargo workspace version {expected}"
            )
        result = version
    elif args.command == "package":
        result = package(args.binary, args.target, args.version, args.output_dir)
    else:
        result = write_checksums(args.dist_dir, args.output)
    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
