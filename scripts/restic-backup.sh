#!/bin/sh
# Nightly restic backup of $HOME, driven by restic-backup.timer.
#
# Runs as the USER, deliberately: the old backup-system.sh needed pkexec (so it
# could never run unattended) and, because it used `tar --one-file-system` from
# /, it silently archived only the root btrfs subvolume — /home has a distinct
# st_dev, so none of the user's data was ever in it. This backs up exactly the
# data that one missed.
#
# Writes the SAME status file the settings app's Storage page reads
# (last_backup_time / backup_size / error_message), so that page reports these
# runs with no change to it.
#
# Credentials live in ~/.config/restic/env (mode 600, NOT versioned): the
# repository URL, the backend's keys, and RESTIC_PASSWORD_FILE. Nothing secret
# belongs in this file.

set -eu

STATUS_FILE="${1:-$HOME/.config/cce/backup_status.txt}"
ENV_FILE="${RESTIC_ENV_FILE:-$HOME/.config/restic/env}"

PREV_TIME="Never"
PREV_SIZE="0 B"
read_previous() {
    [ -f "$STATUS_FILE" ] || return 0
    PREV_TIME=$(sed -n 's/^last_backup_time *= *//p' "$STATUS_FILE" | head -1)
    PREV_SIZE=$(sed -n 's/^backup_size *= *//p' "$STATUS_FILE" | head -1)
    [ -n "$PREV_TIME" ] || PREV_TIME="Never"
    [ -n "$PREV_SIZE" ] || PREV_SIZE="0 B"
}

write_status() {
    mkdir -p "$(dirname "$STATUS_FILE")"
    cat > "$STATUS_FILE" <<EOF
last_backup_time = $1
backup_size = $2
error_message = ${3:-}
EOF
}

# Preserve the last good run's time and size on failure, so one bad night does
# not blank a good backup's record (the same rule backup-system.sh follows).
fail() {
    read_previous
    write_status "$PREV_TIME" "$PREV_SIZE" "$1"
    echo "Error: $1" >&2
    exit 1
}

[ -f "$ENV_FILE" ] || fail "No restic env file at $ENV_FILE"
# shellcheck disable=SC1090
. "$ENV_FILE"
[ -n "${RESTIC_REPOSITORY:-}" ] || fail "RESTIC_REPOSITORY not set in $ENV_FILE"
[ -n "${RESTIC_PASSWORD_FILE:-}${RESTIC_PASSWORD:-}" ] || \
    fail "Neither RESTIC_PASSWORD_FILE nor RESTIC_PASSWORD set in $ENV_FILE"

restic snapshots --no-lock >/dev/null 2>&1 || \
    fail "Cannot reach or unlock the restic repository ($RESTIC_REPOSITORY)"

# --exclude-caches honours CACHEDIR.TAG, which cargo writes into every target/
# dir — that alone drops the ~432k build-artifact files. The explicit excludes
# cover the churny paths that carry no tag.
restic backup "$HOME" \
    --tag nightly \
    --exclude-caches \
    --exclude "$HOME/.cache" \
    --exclude "$HOME/.dropbox" \
    --exclude "$HOME/.dropbox-dist" \
    --exclude "$HOME/.local/state/cce-shadow" \
    --exclude "$HOME/.local/share/Trash" \
    --exclude "**/target/debug" \
    --exclude "**/target/release" \
    --exclude "**/node_modules" \
    --exclude "**/__pycache__" \
    --exclude "**/.venv" \
    || fail "restic backup failed (see journalctl --user -u restic-backup)"

# Retention. Runs after a successful backup only, so a failed night never
# prunes anything.
restic forget --tag nightly \
    --keep-daily 7 --keep-weekly 4 --keep-monthly 12 \
    --prune >/dev/null 2>&1 || echo "Warning: forget/prune failed" >&2

SIZE_STR=$(restic stats --mode raw-data 2>/dev/null \
    | sed -n 's/.*Total Size: *//p' | head -1)
[ -n "$SIZE_STR" ] || SIZE_STR="unknown"

write_status "$(date '+%Y-%m-%d %H:%M:%S')" "$SIZE_STR" ""
echo "Backup completed successfully ($SIZE_STR in repo)"
