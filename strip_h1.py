#!/usr/bin/env python3
"""
Strip shared document-level H1 headers from spec files and promote headings.

Processes all .md files in language-spec/spec/ that start with one of:
  - "# 1. Writ Language Specification"
  - "# Writ IL Specification"
  - "# Appendix"

For each matching file:
1. Removes the first line (shared H1)
2. Removes any blank lines immediately following
3. Promotes all heading levels by one (## -> #, ### -> ##, etc.)
4. Writes the file back in-place
"""

import sys
import re
import os
import glob

SHARED_H1_PATTERNS = [
    "# 1. Writ Language Specification",
    "# Writ IL Specification",
    "# Appendix",
]

def process_file(path):
    with open(path, 'r', encoding='utf-8') as fh:
        lines = fh.readlines()

    if not lines:
        return False

    first_line = lines[0].rstrip('\n').rstrip('\r')
    if first_line not in SHARED_H1_PATTERNS:
        return False

    # Drop first line (shared H1)
    lines = lines[1:]
    # Drop leading blank lines
    while lines and lines[0].strip() == '':
        lines = lines[1:]
    # Promote heading levels: ## -> #, ### -> ##, etc.
    result = []
    for line in lines:
        m = re.match(r'^(#{2,})(.*)', line)
        if m:
            result.append('#' * (len(m.group(1)) - 1) + m.group(2) + '\n')
        else:
            result.append(line)

    with open(path, 'w', encoding='utf-8', newline='') as fh:
        fh.writelines(result)

    return True

def main():
    spec_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'language-spec', 'spec')
    files = sorted(glob.glob(os.path.join(spec_dir, '*.md')))
    processed = 0
    skipped = 0
    for f in files:
        if process_file(f):
            print(f"PROCESSED: {os.path.basename(f)}")
            processed += 1
        else:
            print(f"  SKIPPED: {os.path.basename(f)}")
            skipped += 1
    print(f"\nDone: {processed} processed, {skipped} skipped (already stripped or no matching H1).")

if __name__ == '__main__':
    main()
