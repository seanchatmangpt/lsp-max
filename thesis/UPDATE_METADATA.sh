#!/bin/bash
# UPDATE_METADATA.sh — Quick script to fill in author/institution placeholders
#
# Usage:
#   ./UPDATE_METADATA.sh "Your Name" "Your Department" "Your Institution"
#   Then: latexmk -pdf -bibtex- thesis.tex

set -e

if [ $# -ne 3 ]; then
  cat <<EOF
Usage: $0 "Author Name" "Department" "Institution"

Example:
  $0 "Jane Doe" "Process Intelligence Group" "RWTH Aachen University"

This will update thesis.tex and recompile the PDF.
EOF
  exit 1
fi

AUTHOR="$1"
DEPT="$2"
INST="$3"

echo "Updating metadata..."
echo "  Author: $AUTHOR"
echo "  Department: $DEPT"
echo "  Institution: $INST"

# Update \author{} on line 148
sed -i "s/\\\\author{.*}/\\\\author{$AUTHOR}/g" thesis.tex

# Update author name on title page (line 170)
sed -i "s/{\\\\large \\[Author Name\\]}/{\\\\large $AUTHOR}/g" thesis.tex

# Update department and institution (line 172)
sed -i "s/{\\\\normalsize \\[Department \\\/ Doctoral School\\]}/{\\\\normalsize $DEPT}/g" thesis.tex
sed -i "s/{\\\\lbrace\\[Institution\\]\\\\rbrace}/{$INST}/g" thesis.tex

echo "Metadata updated. Recompiling..."
latexmk -pdf -bibtex- thesis.tex

echo ""
echo "✓ Done. PDF ready: thesis.pdf"
