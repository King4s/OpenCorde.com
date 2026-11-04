#!/usr/bin/env python3
"""Verify the entire published OpenMLS crate plus exactly two import changes."""
import argparse
import hashlib
import io
from pathlib import Path, PurePosixPath
import tarfile
import urllib.request

ARCHIVE_SHA256 = "95d4a03899704c17e7b280676a3301216faccf9b19eb860968f84a492aef944b"
LICENSE_SHA256 = "43e5e3c4b5cca67f9ea912f7e1929702a848aa765d3b3e14f25d3838a5a5565d"
ARCHIVE_URL = "https://static.crates.io/crates/openmls/openmls-0.5.0.crate"
PATCHES = {
    "src/group/group_context.rs": (
        b"framing::{mls_auth_content::AuthenticatedContent, ConfirmedTranscriptHashInput},",
        b"framing::{\n        mls_auth_content::AuthenticatedContent, public_message::ConfirmedTranscriptHashInput,\n    },",
    ),
    "src/group/public_group/mod.rs": (
        b"framing::InterimTranscriptHashInput,",
        b"framing::public_message::InterimTranscriptHashInput,",
    ),
}


def verify(archive, vendor):
    if hashlib.sha256(archive).hexdigest() != ARCHIVE_SHA256:
        raise ValueError("Published archive checksum mismatch")
    expected_paths = {"LICENSE"}
    applied = set()
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as package:
        for member in package.getmembers():
            if member.isdir():
                continue
            if not member.isfile():
                raise ValueError(f"Unexpected archive entry: {member.name}")
            relative = PurePosixPath(member.name).relative_to("openmls-0.5.0")
            if ".." in relative.parts:
                raise ValueError("Unsafe archive path")
            name = str(relative)
            expected_paths.add(name)
            expected = package.extractfile(member).read()
            if name in PATCHES:
                before, after = PATCHES[name]
                if expected.count(before) != 1:
                    raise ValueError(f"Patch precondition failed: {name}")
                expected = expected.replace(before, after)
                applied.add(name)
            if (vendor / name).is_symlink() or (vendor / name).read_bytes() != expected:
                raise ValueError(f"Unexpected vendor modification: {name}")
    if applied != set(PATCHES):
        raise ValueError("A required patch was not verified")
    actual_paths = {str(p.relative_to(vendor)).replace("\\", "/")
                    for p in vendor.rglob("*") if p.is_file() or p.is_symlink()}
    if actual_paths != expected_paths:
        raise ValueError(f"Vendor file set differs: {actual_paths ^ expected_paths}")
    if hashlib.sha256((vendor / "LICENSE").read_bytes()).hexdigest() != LICENSE_SHA256:
        raise ValueError("Upstream license checksum mismatch")
    print(f"Verified {len(expected_paths)} files: published OpenMLS 0.5.0 plus two import fixes and upstream license")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="Use a cached .crate archive (offline)")
    args = parser.parse_args()
    archive = (args.archive.read_bytes() if args.archive else
               urllib.request.urlopen(ARCHIVE_URL, timeout=60).read())
    verify(archive, Path(__file__).resolve().parents[1] / "vendor/openmls-0.5.0")


if __name__ == "__main__":
    main()
