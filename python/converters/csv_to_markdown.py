#!/usr/bin/env python3
"""
python/converters/csv_to_markdown.py

Convert a CSV file into a GitHub-ready Markdown table.

The converter:
- Assumes RFC-4180-like CSVs (comma-delimited, quoted as needed).
- Uses pandas for CSV loading and tabulate for Markdown rendering.
- Does NOT attempt to "fix" broken CSVs; validate first.
"""

from __future__ import annotations

import sys
import pathlib
from typing import Optional

import pandas as pd
from tabulate import tabulate
import click


def _load_csv(path: pathlib.Path) -> pd.DataFrame:
    """
    Load a CSV file with explicit, RFC-like parameters.
    """
    return pd.read_csv(
        path,
        encoding="utf-8",
        sep=",",
        engine="python",
        quotechar='"',
        escapechar=None,
    )


@click.command()
@click.argument("csv_path", type=click.Path(exists=True, dir_okay=False))
@click.option(
    "--max-rows",
    type=int,
    default=100,
    show_default=True,
    help="Maximum number of rows to include in the Markdown table.",
)
@click.option(
    "--output",
    "-o",
    type=click.Path(dir_okay=False),
    default="-",
    help="Output file path, or '-' for stdout.",
    show_default=True,
)
def main(csv_path: str, max_rows: int, output: str) -> None:
    """
    Convert CSV_PATH to a Markdown table.

    Example:
      python converters/csv_to_markdown.py examples/valid_example.csv -o table.md
    """
    path = pathlib.Path(csv_path)

    try:
        df = _load_csv(path)
    except Exception as exc:  # noqa: BLE001
        click.echo(f"[csv-to-md] Failed to load CSV: {exc}", err=True)
        sys.exit(2)

    if max_rows > 0:
        df = df.head(max_rows)

    md_table = tabulate(df, headers="keys", tablefmt="github", showindex=False)

    if output == "-" or output == "":
        click.echo(md_table)
    else:
        out_path = pathlib.Path(output)
        out_path.write_text(md_table + "\n", encoding="utf-8")
        click.echo(f"[csv-to-md] Wrote Markdown table to {out_path}")

    sys.exit(0)


if __name__ == "__main__":
    main()
