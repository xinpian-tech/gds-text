#!/usr/bin/env bash
# Run sky130 met1 DRC on a gds-text export and print a summary.
set -euo pipefail

IN="${1:-/tmp/gds-out.gds}"
OUT="${2:-/tmp/gds-out.lyrdb}"

if [ ! -f "$IN" ]; then
    echo "input GDS not found: $IN"
    exit 1
fi

# klayout's DRC engine touches Qt; spin up Xvfb so a real X display is
# available even on a headless box.
DISPLAY_NUM=:99
cleanup() { [ -n "${XPID:-}" ] && kill "$XPID" 2>/dev/null || true; }
trap cleanup EXIT
Xvfb $DISPLAY_NUM -screen 0 1024x768x24 -ac >/dev/null 2>&1 &
XPID=$!
sleep 1
export DISPLAY=$DISPLAY_NUM

echo "running klayout DRC on $IN"
klayout -b -r scripts/sky130_met1.drc -rd in="$IN" -rd out="$OUT"

# Count violation items in the report XML.
VIOLATIONS=$(grep -c '<item>' "$OUT" || true)
echo
echo "report:    $OUT"
echo "violations: $VIOLATIONS"

if [ "$VIOLATIONS" -gt 0 ]; then
    echo "DRC FAIL"
    exit 1
fi
echo "DRC PASS"
