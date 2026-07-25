from __future__ import annotations

import os
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
import zipfile


sys.path.insert(0, str(Path(__file__).resolve().parent))
from package_release import (
    normalize_version,
    package,
    sha256,
    workspace_version,
    write_checksums,
)


class PackageReleaseTests(unittest.TestCase):
    def test_windows_and_posix_archives_are_deterministic_and_complete(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / "skadi-cli"
            binary.write_bytes(b"test-skadi-binary")
            version = workspace_version()

            first = root / "first"
            second = root / "second"
            windows_one = package(binary, "x86_64-pc-windows-msvc", version, first)
            windows_two = package(binary, "x86_64-pc-windows-msvc", version, second)
            linux_one = package(binary, "x86_64-unknown-linux-musl", version, first)
            linux_two = package(binary, "x86_64-unknown-linux-musl", version, second)

            self.assertEqual(sha256(windows_one), sha256(windows_two))
            self.assertEqual(sha256(linux_one), sha256(linux_two))

            expected_root = f"skadi-cli-v{version}-x86_64-pc-windows-msvc"
            with zipfile.ZipFile(windows_one) as archive:
                names = set(archive.namelist())
                self.assertIn(f"{expected_root}/skadi-cli.exe", names)
                self.assertIn(f"{expected_root}/VERSION", names)
                self.assertIn(f"{expected_root}/LICENSE", names)
                self.assertIn(f"{expected_root}/README.md", names)
                self.assertIn(f"{expected_root}/CHANGELOG.md", names)

            expected_root = f"skadi-cli-v{version}-x86_64-unknown-linux-musl"
            with tarfile.open(linux_one, "r:gz") as archive:
                names = set(archive.getnames())
                self.assertIn(f"{expected_root}/skadi-cli", names)
                mode = archive.getmember(f"{expected_root}/skadi-cli").mode
                self.assertEqual(mode, 0o755)

    def test_checksum_manifest_is_sorted_and_verifiable(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "b.zip").write_bytes(b"b")
            (root / "a.tar.gz").write_bytes(b"a")
            output = write_checksums(root, root / "SHA256SUMS")
            lines = output.read_text(encoding="ascii").splitlines()
            self.assertTrue(lines[0].endswith("  a.tar.gz"))
            self.assertTrue(lines[1].endswith("  b.zip"))

    def test_package_rejects_version_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / "skadi-cli"
            binary.write_bytes(b"binary")
            with self.assertRaisesRegex(ValueError, "does not match"):
                package(binary, "x86_64-unknown-linux-musl", "9.9.9", root / "dist")

    def test_version_normalization_accepts_tag_prefix(self) -> None:
        version = workspace_version()
        self.assertEqual(normalize_version(f"v{version}"), version)


if __name__ == "__main__":
    os.environ.setdefault("SOURCE_DATE_EPOCH", "0")
    unittest.main()
