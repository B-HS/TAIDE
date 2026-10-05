#!/bin/sh
set -eu

SPIKE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SPIKE_CANDIDATE=${1:-}
case "$SPIKE_CANDIDATE" in
    egui) SPIKE_DISPLAY_NAME='TAIDE M8 Egui Spike' ;;
    iced) SPIKE_DISPLAY_NAME='TAIDE M8 Iced Spike' ;;
    *) printf '%s\n' 'usage: package-spike.sh egui|iced' >&2; exit 1 ;;
esac

SPIKE_EXECUTABLE="taide-$SPIKE_CANDIDATE-spike"
SPIKE_BUNDLE="$SPIKE_ROOT/target/$SPIKE_DISPLAY_NAME.app"
test -f "$SPIKE_ROOT/target/debug/$SPIKE_EXECUTABLE"
plutil -lint "$SPIKE_ROOT/bundle/$SPIKE_CANDIDATE-Info.plist"
mkdir -p "$SPIKE_BUNDLE/Contents/MacOS"
cp "$SPIKE_ROOT/bundle/$SPIKE_CANDIDATE-Info.plist" "$SPIKE_BUNDLE/Contents/Info.plist"
cp "$SPIKE_ROOT/target/debug/$SPIKE_EXECUTABLE" "$SPIKE_BUNDLE/Contents/MacOS/$SPIKE_EXECUTABLE"
codesign --force --sign - "$SPIKE_BUNDLE"
codesign --verify --strict "$SPIKE_BUNDLE"
printf '%s\n' "$SPIKE_BUNDLE"
