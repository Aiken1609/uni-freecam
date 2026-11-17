#!/usr/bin/env python3
"""Build helper for Universal Freecam.

    python tools/pack.py stage                 # assemble build/legacy/uni_freecam (bundle + core)
    python tools/pack.py package-legacy        # build/uni_freecam-<version>.hnmod for the legacy route
    python tools/pack.py package-native [--horizon SHA] [--mixins SHA]
                                               # build/uni_freecam-<version>.hmod for the typed-API route
    python tools/pack.py apply --horizon DIR --mixins-tool BIN --out DIR
                                               # apply the bundle to a Horizon checkout (what the launcher does)
    python tools/pack.py check --stage DIR [--target-dir DIR]
                                               # cargo check the game that `apply` staged

The core (core/freecam_core.rs) is the single source of the camera logic. Each adapter
gets a copy made at stage time, so the copies can never drift apart.
"""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE = ROOT / "core" / "freecam_core.rs"
COMPAT = json.loads((ROOT / "compat.json").read_text(encoding="utf-8"))
LEGACY_SRC = ROOT / "legacy" / COMPAT["id"]
BUILD = ROOT / "build"
NATIVE_SRC = ROOT / "native"
SHA = re.compile(r"^[0-9a-f]{40}$")


def manifest_value(path, key):
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith(f"{key} = "):
            return line.split("=", 1)[1].strip().strip('"')
    raise SystemExit(f"{key} missing in {path}")


def core_version():
    for line in CORE.read_text(encoding="utf-8").splitlines():
        if line.startswith("pub const VERSION"):
            return line.split('"')[1]
    raise SystemExit("VERSION missing in the core")


def verify_versions():
    manifest = LEGACY_SRC / "horizon-mixin.toml"
    versions = {
        "compat.json": COMPAT["version"],
        "core": core_version(),
        "legacy manifest": manifest_value(manifest, "version"),
    }
    if len(set(versions.values())) != 1:
        raise SystemExit(f"versions differ: {versions}")
    if manifest_value(manifest, "horizon_revision") != COMPAT["legacy"]["horizon_revision"]:
        raise SystemExit("horizon_revision differs between horizon-mixin.toml and compat.json")


ITEM = re.compile(r"^(pub |impl|static |fn |const |struct |enum )")


def module_payload(text):
    """The core as the body of a support module: items only, as horizon-mixins requires.

    A module may not carry file attributes (`//!`, `#![..]`), so those are dropped, the host-only
    tests are cut, and each top-level item gets its own `allow(dead_code)` since a game module
    would otherwise warn about every part of the core this adapter does not call.
    """
    cut = text.find("\n#[cfg(test)]\nmod tests {")
    if cut != -1:
        text = text[:cut] + "\n"
    out = []
    for line in text.splitlines():
        if line.startswith("//!") or line.startswith("#!["):
            continue
        if ITEM.match(line):
            out.append("#[allow(dead_code)]")
        out.append(line)
    return "\n".join(out).lstrip("\n") + "\n"


def stage():
    verify_versions()
    out = BUILD / "legacy" / COMPAT["id"]
    if out.exists():
        shutil.rmtree(out)
    shutil.copytree(LEGACY_SRC, out)
    (out / "runtime.rs").write_text(module_payload(CORE.read_text(encoding="utf-8")), encoding="utf-8")
    return out


def package_legacy():
    bundle = stage()
    info = {
        "schema": 1,
        "id": COMPAT["id"],
        "version": COMPAT["version"],
        "game": COMPAT["game"],
        "horizon": COMPAT["legacy"]["horizon_revision"],
        "mixins": COMPAT["legacy"]["horizon_mixins_revision"],
    }
    target = BUILD / f"{COMPAT['id']}-{COMPAT['version']}.hnmod"
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("horizon-native.json", json.dumps(info, indent=2) + "\n")
        for file in sorted(bundle.rglob("*")):
            if file.is_file():
                archive.write(file, f"mixins/{COMPAT['id']}/{file.relative_to(bundle).as_posix()}")
    digest = hashlib.sha256(target.read_bytes()).hexdigest()
    print(f"{target}\nsha256 {digest}")
    return target


def package_native(horizon, mixins):
    """A .hmod for the typed-API route: manifest with the chosen pins, Cargo.toml, src/, assets/."""
    horizon = horizon or COMPAT["native"]["horizon_revision"]
    mixins = mixins or COMPAT["native"]["horizon_mixins_revision"]
    for name, value in (("horizon", horizon), ("mixins", mixins)):
        if not SHA.match(value):
            raise SystemExit(f"--{name} must be 40 lowercase hex characters, got {value!r}")
    manifest = (NATIVE_SRC / "horizon-mod.toml").read_text(encoding="utf-8")
    manifest = manifest.replace("@HORIZON@", horizon).replace("@MIXINS@", mixins)
    version = manifest_value(NATIVE_SRC / "horizon-mod.toml", "version")
    if version != COMPAT["version"] or version != core_version():
        raise SystemExit("version differs between native manifest, compat.json and the core")
    target = BUILD / f"{COMPAT['id']}-{version}.hmod"
    BUILD.mkdir(exist_ok=True)
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("horizon-mod.toml", manifest)
        archive.write(NATIVE_SRC / "Cargo.toml", "Cargo.toml")
        archive.write(NATIVE_SRC / "src" / "lib.rs", "src/lib.rs")
        archive.write(CORE, "src/freecam_core.rs")
        for file in sorted((NATIVE_SRC / "assets").rglob("*")):
            if file.is_file():
                archive.write(file, file.relative_to(NATIVE_SRC).as_posix())
    print(f"{target}\nsha256 {hashlib.sha256(target.read_bytes()).hexdigest()}")
    return target


def apply(horizon, mixins_tool, out):
    bundle = stage()
    with tempfile.TemporaryDirectory() as scratch:
        mods = pathlib.Path(scratch) / "mods"
        shutil.copytree(bundle, mods / COMPAT["id"])
        # horizon-mixins refuses an existing stage without its marker, and a fresh one is the
        # reliable choice on Windows, so stage beside the target and copy over what changed.
        fresh = out.with_name(out.name + ".fresh")
        if fresh.exists():
            shutil.rmtree(fresh)
        subprocess.run(
            [str(mixins_tool), "apply", "--workspace", str(horizon), "--mods", str(mods), "--out", str(fresh)],
            check=True,
        )
    for source in fresh.rglob("*"):
        if source.is_file():
            target = out / source.relative_to(fresh)
            if not target.exists() or target.read_bytes() != source.read_bytes():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, target)
    shutil.rmtree(fresh)


def check(stage_dir, target_dir):
    command = ["cargo", "check", "--locked", "--offline", "-p", "horizon-client", "--message-format", "short"]
    if target_dir:
        command += ["--target-dir", str(target_dir)]
    subprocess.run(command, cwd=stage_dir, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("stage")
    sub.add_parser("package-legacy")
    n = sub.add_parser("package-native")
    n.add_argument("--horizon")
    n.add_argument("--mixins")
    a = sub.add_parser("apply")
    a.add_argument("--horizon", required=True, type=pathlib.Path)
    a.add_argument("--mixins-tool", required=True, type=pathlib.Path)
    a.add_argument("--out", required=True, type=pathlib.Path)
    c = sub.add_parser("check")
    c.add_argument("--stage", required=True, type=pathlib.Path)
    c.add_argument("--target-dir", type=pathlib.Path)
    args = parser.parse_args()
    if args.command == "stage":
        print(stage())
    elif args.command == "package-legacy":
        package_legacy()
    elif args.command == "package-native":
        package_native(args.horizon, args.mixins)
    elif args.command == "apply":
        apply(args.horizon, args.mixins_tool, args.out)
    else:
        check(args.stage, args.target_dir)


if __name__ == "__main__":
    sys.exit(main())
