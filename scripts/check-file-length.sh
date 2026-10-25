#!/usr/bin/env bash
# Fails if any tracked source or doc file has more than 500 lines.
set -euo pipefail

cd "$(dirname "$0")/.."
LIMIT=500
status=0

while IFS= read -r file; do
  lines=$(wc -l < "$file")
  if [ "$lines" -gt "$LIMIT" ]; then
    echo "$file has $lines lines (limit $LIMIT)"
    status=1
  fi
done < <(git ls-files --cached --others --exclude-standard \
  | grep -Ev '(^|/)(package-lock\.json|Cargo\.lock|LICENSE)$|\.(png|ico|icns)$' \
  | while IFS= read -r f; do [ -f "$f" ] && echo "$f"; done)

exit "$status"
