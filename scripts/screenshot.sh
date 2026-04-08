#!/usr/bin/env bash
# Run gds-text under Xvfb (software GL via GLX) and capture screenshots.
set -euo pipefail

OUT_DIR="${1:-/tmp/gds-text-shots}"
mkdir -p "$OUT_DIR"

DISPLAY_NUM=:99
WIDTH=1280
HEIGHT=800

cleanup() {
    [ -n "${APP_PID:-}" ] && kill "$APP_PID" 2>/dev/null || true
    [ -n "${XVFB_PID:-}" ] && kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

# Mesa path is exposed by the flake dev shell via GDS_TEXT_MESA.
if [ -z "${GDS_TEXT_MESA:-}" ]; then
    echo "error: GDS_TEXT_MESA not set; run this script inside 'nix develop'"
    exit 1
fi
MESA_STORE="$GDS_TEXT_MESA"
echo "mesa: $MESA_STORE"

# Start Xvfb with software GL support.
echo "Starting Xvfb..."
Xvfb $DISPLAY_NUM -screen 0 ${WIDTH}x${HEIGHT}x24 +extension GLX +extension RANDR +render -noreset -ac &
XVFB_PID=$!
sleep 2

export DISPLAY=$DISPLAY_NUM
export LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe
export LIBGL_DRIVERS_PATH="${GDS_TEXT_MESA_DRI_PATH:-$MESA_STORE/lib/dri}"
export __EGL_VENDOR_LIBRARY_FILENAMES="${GDS_TEXT_MESA_EGL_VENDOR:-$MESA_STORE/share/glvnd/egl_vendor.d/50_mesa.json}"
export LD_LIBRARY_PATH="$MESA_STORE/lib:${LD_LIBRARY_PATH:-}"
# Force winit to use X11, and ask glutin/eframe to prefer EGL (mesa provides
# software EGL via GBM even without a real GPU).
export WINIT_UNIX_BACKEND=x11
export EGL_PLATFORM=surfaceless

# Sanity check: make sure we have a working GL.
glxinfo 2>/dev/null | head -3 || echo "(no glxinfo)"

echo "Launching gds-text..."
./target/release/gds-text > "$OUT_DIR/app.log" 2>&1 &
APP_PID=$!

# Wait for the window.
for i in $(seq 1 20); do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
        echo "App died early, see $OUT_DIR/app.log"
        tail -20 "$OUT_DIR/app.log"
        exit 1
    fi
    if xdotool search --name "gds-text" >/dev/null 2>&1; then
        echo "Window appeared after ${i}s"
        break
    fi
    sleep 1
done

sleep 3
import -window root "$OUT_DIR/01_initial.png"
echo "saved 01_initial.png"

# --- Step 2: drag the snippet on the canvas ---
# The snippet is at canvas position (20, 40); canvas origin is around
# (10, 30) at scale ~1.0 since canvas is 800x500 and window is 1280x800.
xdotool mousemove --sync 90 90
sleep 0.4
xdotool mousedown 1
sleep 0.4
# Move in several steps so egui registers drag_started.
for x in 150 220 300 380 460; do
    xdotool mousemove --sync $x 260
    sleep 0.15
done
sleep 0.3
xdotool mouseup 1
sleep 0.8
import -window root "$OUT_DIR/02_after_drag.png"
echo "saved 02_after_drag.png"

# --- Step 3: click the "90deg" rotation preset ---
xdotool mousemove --sync 1080 417
sleep 0.3
xdotool click 1
sleep 0.8
import -window root "$OUT_DIR/03_rotated_90.png"
echo "saved 03_rotated_90.png"

# --- Step 4: bump the font size slider for the snippet ---
xdotool mousemove --sync 1100 375
sleep 0.3
xdotool click 1
sleep 0.3
# Drag the slider handle right.
xdotool mousedown 1
sleep 0.2
xdotool mousemove --sync 1130 375
sleep 0.2
xdotool mouseup 1
sleep 0.8
import -window root "$OUT_DIR/04_bigger_font.png"
echo "saved 04_bigger_font.png"

ls -lh "$OUT_DIR"
