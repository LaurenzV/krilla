#!/bin/bash

# A quick and dirty scripts that rewrites all PDFs in the store with GhostScript and checks
# whether any errors/warnings were emitted.

PDF_DIR="store/snapshots"
had_errors=0

while IFS= read -r -d '' pdf_file; do
    temp_file="${pdf_file}.tmp"
    
    echo "Processing: $pdf_file"
    
    error_output=$(gs -sDEVICE=pdfwrite -dNOPAUSE -dBATCH \
        -sOutputFile="$temp_file" "$pdf_file" 2>&1)
    gs_exit_code=$?
    
    if echo "$error_output" | grep -qi "error"; then
         echo "$error_output"
        had_errors=1
        rm -f "$temp_file"
    fi
    
    echo ""
done < <(find "$PDF_DIR" -type f -name "*.pdf" -print0)

exit $had_errors