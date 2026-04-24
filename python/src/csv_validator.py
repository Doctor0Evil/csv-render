#!/usr/bin/env python3
"""
python/src/csv_validator.py

Lightweight CSV validation script for the csv-render project.

This tool is designed to be:
- Strict about structural integrity (column count consistency).
- Compatible with RFC 4180 assumptions (comma-delimited, quoted fields).
- Safe to use in CI as a quick pre-check, with the Rust csv-core library
  remaining the ultimate oracle for correctness.
"""

from __future__ import annotations

import sys
import pathlib
from typing import List, Tuple

import pandas as pd
import click


def _load_csv(path: pathlib.Path) -> pd.DataFrame:
    """
    Load a CSV file using pandas with strict, explicit parameters.

    This function assumes:
    - UTF-8 encoding.
    - Comma delimiter.
    - RFC-4180-like quoting behavior (handled by pandas' engine).

    It does NOT attempt to fix malformed CSVs; it fails fast.
    """
    return pd.read_csv(
        path,
        encoding="utf-8",
        sep=",",
        engine="python",
        quotechar='"',
        escapechar=None,
    )


def _check_column_consistency(df: pd.DataFrame) -> Tuple[bool, str]:
    """
    Ensure all rows have the same number of columns as the header.

    Because pandas already normalizes rows into a rectangular DataFrame,
    this check focuses on detecting columns that might have been created
    by stray commas or mis-quoted fields (e.g., suspicious unnamed columns).
    """
    # Basic heuristic: any column starting with "Unnamed:" is suspicious.
    suspicious_cols: List[str] = [
        c for c in df.columns if isinstance(c, str) and c.startswith("Unnamed:")
    ]

    if suspicious_cols:
        return (
            False,
            (
                "Detected suspicious columns likely caused by column-count "
                f"mismatches or stray commas: {', '.join(suspicious_cols)}"
            ),
        )

    return True, "All columns appear structurally consistent."


@click.command()
@click.argument("csv_path", type=click.Path(exists=True, dir_okay=False))
def main(csv_path: str) -> None:
    """
    Validate a CSV file for basic structural correctness.

    Exit code:
      0 - CSV appears structurally sound.
      1 - Structural issues detected.
      2 - Fatal error loading the CSV.
    """
    path = pathlib.Path(csv_path)

    try:
        df = _load_csv(path)
    except Exception as exc:  # noqa: BLE001
        click.echo(f"[csv-validator] Failed to load CSV: {exc}", err=True)
        sys.exit(2)

    ok, message = _check_column_consistency(df)
    if not ok:
        click.echo(f"[csv-validator] INVALID: {message}", err=True)
        sys.exit(1)

    click.echo(f"[csv-validator] OK: {message}")
    sys.exit(0)


if __name__ == "__main__":
    main()
