#!/usr/bin/env python3
"""
repair_loop.py

Orchestrates an iterative CSV repair process using:

- csv-cli validate --json
- A Large Language Model (LLM) accessible via a function call

The workflow:

1. Start from an initial CSV file (broken.csv).
2. Run `csv-cli validate --json` on it, capturing error_report.json.
3. If there are errors, call the LLM with (broken.csv, error_report.json)
   to produce a new candidate CSV (fixed.csv).
4. Repeat until validation passes or a maximum number of iterations is reached.

This script is intentionally minimal and focuses on wiring. You are expected
to plug in your own LLM invocation in the `call_llm_repair` function.
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Optional, Tuple


CSV_CLI_BIN = "csv-cli"
MAX_ITERATIONS = 5


def run_validator(csv_path: Path, schema_path: Optional[Path] = None) -> Tuple[bool, dict]:
    """
    Run `csv-cli validate --json` on the given CSV file.

    Returns:
        (is_valid, error_report_dict)
    """
    cmd = [CSV_CLI_BIN, "validate", "--json", str(csv_path)]
    if schema_path is not None:
        cmd.extend(["--schema", str(schema_path)])

    proc = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
    )

    stdout = proc.stdout.strip()
    if not stdout:
        # No JSON produced; treat as no errors if exit code is 0.
        return proc.returncode == 0, {"errors": []}

    try:
        report = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"Failed to parse validator JSON output: {exc}") from exc

    is_valid = proc.returncode == 0
    return is_valid, report


def call_llm_repair(broken_csv: str, error_report: dict) -> str:
    """
    Call an LLM to repair the CSV.

    This is a stub. You must implement the actual call to your preferred model
    (e.g., via HTTP API, local inference, or an SDK).

    The function should:
        - Receive the raw CSV as text (`broken_csv`).
        - Receive the error report as a Python dict (`error_report`).
        - Return the corrected CSV as text.

    For now, this is a placeholder that simply returns the input unchanged.
    """
    # TODO: Replace this stub with a real LLM call.
    # Example (pseudo-code):
    #
    # prompt = RENDER_PROMPT_TEMPLATE(broken_csv, json.dumps(error_report, indent=2))
    # response = llm_client.generate(prompt)
    # return response.csv_body
    return broken_csv


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")


def repair_loop(
    input_csv: Path,
    schema_path: Optional[Path] = None,
    max_iterations: int = MAX_ITERATIONS,
) -> Tuple[bool, int]:
    """
    Run the iterative repair loop.

    Args:
        input_csv: Path to the starting CSV file.
        schema_path: Optional path to a schema file.
        max_iterations: Maximum number of repair attempts.

    Returns:
        (is_valid, iterations_used)
    """
    current_csv_path = input_csv
    iterations = 0

    while iterations < max_iterations:
        is_valid, report = run_validator(current_csv_path, schema_path=schema_path)
        if is_valid:
            return True, iterations

        errors = report.get("errors", [])
        if not errors:
            # No errors reported but validator returned non-zero; treat as fatal.
            raise RuntimeError("Validator returned failure but no errors were reported")

        broken_csv = read_text(current_csv_path)
        fixed_csv = call_llm_repair(broken_csv, report)

        iterations += 1
        repaired_path = current_csv_path.parent / f"{current_csv_path.stem}.repaired.{iterations}.csv"
        write_text(repaired_path, fixed_csv)

        # Next iteration uses the repaired file as input.
        current_csv_path = repaired_path

    return False, iterations


def main(argv: Optional[list] = None) -> int:
    if argv is None:
        argv = sys.argv[1:]

    if not argv:
        print("Usage: repair_loop.py <broken.csv> [schema.yaml]", file=sys.stderr)
        return 1

    csv_path = Path(argv[0])
    schema_path: Optional[Path] = Path(argv[1]) if len(argv) > 1 else None

    if not csv_path.exists():
        print(f"CSV file not found: {csv_path}", file=sys.stderr)
        return 1

    is_valid, iterations = repair_loop(csv_path, schema_path=schema_path)
    if is_valid:
        print(f"Validation passed after {iterations} repair iteration(s).")
        return 0
    else:
        print(
            f"Validation did not converge after {iterations} iteration(s). "
            f"Check the latest repaired CSV for details.",
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
