#!/bin/sh
# Full-system backup, run as root via pkexec from the settings app's Storage
# page. Installed to ~/.local/bin by `ccebuild install` (it ships every crate's
# scripts/ dir); it lived unversioned in ~/.local/share/<app>/helpers/ until
# 2026-08-23, where three renames of this app had left the caller looking for
# it under a name it no longer had.
#
# The status file is passed in as $1 rather than recomputed here. It is the one
# thing both sides must agree on, and they cannot agree by construction: the
# caller resolves it through XDG as the user, while this runs as root under
# pkexec, which scrubs the environment. Passing it makes the contract explicit
# instead of two hardcoded paths that silently drifted apart (they had).
#
# Everything user-specific comes from PKEXEC_UID, the uid pkexec records for
# whoever authenticated — never a hardcoded name or home.

set -eu

STATUS_FILE="${1:?usage: backup-system.sh <status-file>}"

if [ -z "${PKEXEC_UID:-}" ]; then
    echo "Error: not running under pkexec (no PKEXEC_UID)" >&2
    exit 1
fi
RUN_USER=$(getent passwd "$PKEXEC_UID" | cut -d: -f1)
if [ -z "$RUN_USER" ]; then
    echo "Error: cannot resolve uid $PKEXEC_UID to a user" >&2
    exit 1
fi

# Previous values are preserved on failure so a failed run reports its error
# without also blanking the last good backup's time and size.
PREV_TIME="Never"
PREV_SIZE="0 B"
read_previous() {
    [ -f "$STATUS_FILE" ] || return 0
    PREV_TIME=$(sed -n 's/^last_backup_time *= *//p' "$STATUS_FILE" | head -1)
    PREV_SIZE=$(sed -n 's/^backup_size *= *//p' "$STATUS_FILE" | head -1)
    [ -n "$PREV_TIME" ] || PREV_TIME="Never"
    [ -n "$PREV_SIZE" ] || PREV_SIZE="0 B"
}

# Written as root, so hand it back to the invoking user — the app reads it
# unprivileged on its next refresh.
write_status() {
    mkdir -p "$(dirname "$STATUS_FILE")"
    cat > "$STATUS_FILE" <<EOF
last_backup_time = $1
backup_size = $2
error_message = ${3:-}
EOF
    chown "$PKEXEC_UID" "$STATUS_FILE" 2>/dev/null || true
    chown "$PKEXEC_UID" "$(dirname "$STATUS_FILE")" 2>/dev/null || true
}

fail() {
    read_previous
    write_status "$PREV_TIME" "$PREV_SIZE" "$1"
    echo "Error: $1" >&2
    exit 1
}

# 1. Destination: /mnt/usb, else the first real mount under the user's media dir
DEST="/mnt/usb"
if ! mountpoint -q "$DEST"; then
    MEDIA_DIR="/run/media/$RUN_USER"
    DEST=""
    if [ -d "$MEDIA_DIR" ]; then
        for d in "$MEDIA_DIR"/*; do
            if [ -d "$d" ] && mountpoint -q "$d"; then
                DEST="$d"
                break
            fi
        done
    fi
    [ -n "$DEST" ] || fail "No external drive mounted at /mnt/usb or $MEDIA_DIR/*"
fi

# Archive name kept as-is deliberately: renaming it to match the app's current
# name would orphan any archive already sitting on the drive rather than
# overwriting it.
ARCHIVE_PATH="$DEST/clear-system-backup.tar.gz"
echo "Starting full system backup to $ARCHIVE_PATH..."

# --one-file-system keeps tar out of virtual mounts and other drives; the
# destination is excluded so the archive cannot recurse into itself.
tar --one-file-system \
    --exclude="/lost+found" \
    --exclude="$DEST" \
    -czf "$ARCHIVE_PATH" \
    -C / . || fail "Backup archive creation failed"

SIZE_STR=$(du -sh "$ARCHIVE_PATH" | awk '{print $1}')
DATE_STR=$(date "+%Y-%m-%d %H:%M:%S")
write_status "$DATE_STR" "$SIZE_STR" ""
echo "Backup completed successfully!"
