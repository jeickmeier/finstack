"""Audit public Rust contract types for effective serde and schema support.

The scanner is a conservative, module-aware Rust lexer rather than a full
compiler frontend. It follows file and inline module visibility, public
re-exports, public type aliases, qualified/imported crate-wide manual impls,
ordinary derives, and ``cfg_attr(..., derive(...))``.

Limitations are deliberate and fail-closed for the maintained contract set:
macro-generated declarations are not expanded, ambiguous unqualified impl
targets are not guessed, and Cargo cfg predicates are evaluated for the
effective default build on the current Python host. ``unix``, ``target_arch``,
and ``target_os`` therefore follow the host running the audit; unknown target
predicates are disabled conservatively. Every maintained contract has an exact
crate/path/type registry entry; if module or syntax handling fails to discover
one, check mode reports the missing registry entry instead of silently reducing
coverage.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from .models import CAPABILITIES, AuditConfigurationError, AuditReport, PublicType
from .scanner import audit_workspace


def _format_capabilities(declaration: PublicType) -> str:
    return ",".join(capability for capability in CAPABILITIES if capability in declaration.capabilities) or "none"


def print_report(report: AuditReport, *, verbose: bool) -> None:
    """Print diagnostics and stable per-crate counts."""
    if verbose:
        print("Contract-like public types:")
        for declaration in report.declarations:
            if declaration.identity in report.contract_identities:
                print(
                    f"  {declaration.path}:{declaration.line}: {declaration.name} [{_format_capabilities(declaration)}]"
                )
        print("Reviewed exceptions:")
        for entry in report.reviewed_exceptions:
            print(f"  {entry.crate}/{entry.path}: {entry.type_name} [{entry.category}] {entry.rationale}")
    for diagnostic in report.diagnostics:
        print(f"{diagnostic.path}:{diagnostic.line}: {diagnostic.type_name}: missing {', '.join(diagnostic.missing)}")
    for stale in report.stale_exceptions:
        actual = (
            "declaration-missing" if stale.actual_missing is None else ",".join(sorted(stale.actual_missing)) or "none"
        )
        allowed = ",".join(sorted(stale.allowed_missing)) or "none"
        entry = stale.entry
        print(
            f"STALE classification {entry.crate}/{entry.path}: {entry.type_name} "
            f"[{entry.category}] reason={stale.reason} actual_missing={actual} "
            f"allowed_missing={allowed}"
        )
    print("Per-crate summary:")
    for summary in report.summaries:
        print(
            f"  {summary.crate}: public={summary.public_types} "
            f"contract={summary.contract_types} exceptions={summary.reviewed_exceptions} "
            f"failures={summary.failures}"
        )
    print(
        f"Total: public={len(report.declarations)} "
        f"contract={sum(item.contract_types for item in report.summaries)} "
        f"exceptions={len(report.reviewed_exceptions)} "
        f"failures={len(report.diagnostics)} stale={len(report.stale_exceptions)}"
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    parser = argparse.ArgumentParser(prog="python -m scripts.serde_audit", description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repository root")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--report", action="store_true", help="show all gated types and reviewed exceptions")
    mode.add_argument("--check", action="store_true", help="fail on missing capabilities or stale exceptions")
    return parser


def main() -> int:
    """Run report mode by default or enforce the audit with ``--check``."""
    parser = build_parser()
    arguments = parser.parse_args()
    try:
        report = audit_workspace(arguments.root)
    except AuditConfigurationError as error:
        parser.error(str(error))
    print_report(report, verbose=arguments.report or not arguments.check)
    return int(arguments.check and report.failed)
