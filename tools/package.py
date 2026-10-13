"""Stage a Linux POC package without downloading, installing or deploying anything."""
import argparse
from pathlib import Path
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    data = args.binary.read_bytes()
    if len(data) < 64 or data[:6] != b"\x7fELF\x02\x01" or data[18:20] not in (b"\x3e\x00", b"\xb7\x00"):
        parser.error("Expected a 64-bit Linux ELF binary for amd64 or arm64")
    if not args.output.parent.is_dir():
        parser.error("Output parent must exist")
    root = Path(__file__).resolve().parents[1]
    if args.output.exists():
        parser.error("Refusing to overwrite an existing package")
    with tarfile.open(args.output, "x:gz") as archive:
        for name in ("manifest.toml", "README.md", "conf", "scripts", "lib", "profiles"):
            archive.add(root / name, arcname=name, filter=clean)
        info = tarfile.TarInfo("dist/arcenal-equipe")
        info.size = len(data)
        info.mode = 0o755
        with args.binary.open("rb") as binary:
            archive.addfile(info, binary)
    print(f"Candidate POC staged: {args.output}")


def clean(info):
    if "__pycache__" in info.name or info.name.endswith(".pyc"):
        return None
    if info.name.startswith("scripts/"):
        info.mode = 0o755
    info.uid = info.gid = 0
    info.uname = info.gname = "root"
    return info


if __name__ == "__main__":
    main()
