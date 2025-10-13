#!/usr/bin/env python3

"""
A script that rewrites all PDFs in the store with GhostScript and checks
whether any errors/warnings were emitted.
"""

import subprocess
import sys
from pathlib import Path
from concurrent.futures import ProcessPoolExecutor, as_completed
import os


def process_pdf(pdf_path):
    print(f"Processing: {pdf_path}")

    try:
        result = subprocess.run(
            [
                "gs",
                "-sDEVICE=pdfwrite",
                "-dNOPAUSE",
                "-dBATCH",
                "-sOutputFile=/dev/null",
                str(pdf_path),
            ],
            capture_output=True,
            text=True,
        )

        # Combine stdout and stderr
        output = result.stdout + result.stderr

        # Check if "error" appears in the output (case-insensitive)
        if "error" in output.lower():
            print(output)
            return (pdf_path, False)

        return (pdf_path, True)

    except Exception as e:
        print(f"Exception processing {pdf_path}: {e}")
        return (pdf_path, False)


def main():
    pdf_dir = Path("store/")

    pdf_files = list(pdf_dir.rglob("*.pdf"))

    if not pdf_files:
        print("No PDF files found")
        return 0

    had_errors = False

    max_workers = os.cpu_count() or 1

    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        futures = {executor.submit(process_pdf, pdf): pdf for pdf in pdf_files}

        for future in as_completed(futures):
            pdf_path, success = future.result()
            if not success:
                had_errors = True

    return 1 if had_errors else 0


if __name__ == "__main__":
    sys.exit(main())
