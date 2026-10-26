#!/bin/sh
# A pretend adb for tests and demos: it serves a fake player kept in $FAKE_ADB_DIR.
# The folder holds remote/ (the player's files, e.g. remote/usr/data/...), devices.txt, fb, ps
# and dmesg. Point the app at it with COMPAS_ADB=scripts/fake-adb.sh. Every call is logged
# to $FAKE_ADB_DIR/calls.log.
D="${FAKE_ADB_DIR:-@DIR@}"
echo "$*" >> "$D/calls.log"
[ "$1" = "-s" ] && shift 2
cmd="$1"; shift
R="$D/remote"
map() { echo "$1" | sed -E "s#(^|[ '=])/(usr|data|sys|proc|tmp|dev|run)/#\1$R/\2/#g"; }
case "$cmd" in
  devices) cat "$D/devices.txt" ;;
  version) echo "Android Debug Bridge version 1.0.41" ;;
  pull)
    src="$R$1"; dst="$2"
    if [ -d "$src" ]; then
      if [ -d "$dst" ]; then cp -R "$src" "$dst/"; else cp -R "$src" "$dst"; fi
    else cp "$src" "$dst"; fi ;;
  push)
    dst="$R$2"; mkdir -p "$(dirname "$dst")"
    if [ -e "$D/corrupt_push" ]; then echo junk > "$dst"; else cp "$1" "$dst"; fi ;;
  shell|exec-out)
    c="$1"
    case "$c" in
      /usr/bin/bootmode.sh*)
        [ -e "$D/bootmode_fails" ] && { echo "/usr/bin/bootmode.sh: not found" >&2; exit 1; }
        # The helper sets the recovery flag in the NAND flag block; `bootmode_noop` pretends it did not.
        [ -e "$D/bootmode_noop" ] || { mkdir -p "$R/dev" && printf 'ota:kernel2' > "$R/dev/mtd5"; }
        echo "ok" ;;
      sync) : ;;
      reboot)
        # `devices_after_reboot` is the device list once the player is back (a new serial, say).
        [ -e "$D/devices_after_reboot" ] && cp "$D/devices_after_reboot" "$D/devices.txt"
        # A real reboot gives the kernel a new boot id; `no_reboot` pretends the command did nothing.
        [ -e "$D/no_reboot" ] || { [ -e "$R/proc/sys/kernel/random/boot_id" ] && echo "boot-$$-$(date +%s)" > "$R/proc/sys/kernel/random/boot_id"; } ;;
      "cat /sys/class/graphics/fb0/virtual_size"*) cat "$D/fb" 2>/dev/null ;;
      ps) cat "$D/ps" 2>/dev/null ;;
      dmesg*) cat "$D/dmesg" 2>/dev/null ;;
      sha256sum*)
        if [ -e "$D/no_sha256sum" ]; then echo "sha256sum: not found" >&2; exit 127; fi
        eval "$(map "$c")" | sed "s#$R##g" ;;
      df*)
        if [ -e "$D/df_output" ]; then cat "$D/df_output"; else
          printf 'Filesystem 1K-blocks Used Available Use%% Mounted on\n/dev/mmcblk0p1 7000000 100 6999900 1%% /data/mnt/sd_0\n'
        fi ;;
      *head\ -c*) eval "$(map "$c")" ;;
      *) eval "$(map "$c")" | sed "s#$R##g" ;;
    esac ;;
  *) echo "fake adb: unknown command $cmd" >&2; exit 1 ;;
esac
