# Plugin store format

Contract implemented by `compas-core::catalog`. Source of truth:
`compas-player/src/network/plugin_store.c`, `store_common.c` and
`src/plugins/plugin_disabled_list.c`. Player runtime details (mutexes, LVGL,
preview RAM cache, USB-gadget write guard) are left out. `PLUGIN_API_VERSION`
is 17 at the time of writing.

Paths below are relative to the card root. Comparisons of destinations are
ASCII case-insensitive because the card is FAT.

## Catalog index

URL: `https://github.com/Starnished66/compas-plugins/releases/latest/download/index.json`
(`PLUGIN_STORE_INDEX_URL`). At most 256 KiB (`STORE_INDEX_LIMIT`). Asset URL:
`https://github.com/Starnished66/compas-plugins/releases/download/<tag>/<asset>`
(`make_asset_url`), where `tag` is the index's `tag`.

### Schema (`plugin_store_parse_index`)

```json
{ "schema": 1, "tag": "v1.2",
  "plugins": [ {
    "id": "my-plugin", "name": "My plugin", "version": "1.2.0", "api_min": 10,
    "description": "...", "category": "...", "author": "...", "size": 1234,
    "preview": { "asset": "p.jpg", "sha256": "<64 hex>", "size": 9000,
                 "width": 100, "height": 150, "variants": [ { ...same... } ] },
    "files": [ { "asset": "my.lua", "dest": ".plugins/my.lua",
                 "sha256": "<64 hex>", "size": 1234, "keep": false } ] } ] }
```

The whole index is rejected on the first violation of any rule. Integers may be
written as integral floats (`15.0`, as `store_json_uint` accepts). Repeated keys
in one object do not fail the index; cJSON uses the first, the desktop app the
last.


| Field                               | Rule                                                                                          |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| `schema`                            | integer, exactly 1                                                                            |
| `tag`                               | token (`[A-Za-z0-9_.-]`), 1 to 63 chars                                                       |
| `plugins`                           | array, at most 200 entries                                                                    |
| `id`                                | `regex_id`: `[a-z0-9][a-z0-9_.-]{0,62}`; unique across the index                              |
| `name`                              | 1 to 64 chars, no control chars (< 32 or 127)                                                 |
| `version`                           | `store_version_valid`: 1 to 31 chars, 1 to 4 numeric groups separated by `.`, no empty groups |
| `description`, `category`, `author` | strings (may be empty), no control chars, at most 400, 32 and 64 chars                        |
| `api_min`                           | integer 1 to 2^32-1                                                                           |
| `size`                              | integer 0 to 2^32-1; must equal the sum of the file sizes                                     |
| `files`                             | array of 1 to 32 entries                                                                      |
| `files[].asset`                     | token (`[A-Za-z0-9_.-]`), 1 to 127 chars                                                      |
| `files[].sha256`                    | exactly 64 lowercase hex chars                                                                |
| `files[].size`                      | integer 1 to 1 MiB (`STORE_FILE_LIMIT`)                                                       |
| `files[].keep`                      | optional boolean, default false                                                               |
| `files[].dest`                      | `plugin_store_dest_valid`, see below                                                          |
| `preview`                           | optional object, see below                                                                    |

Other rules over `files`: exactly one file is the _script_ (`script_dest`: a
`.lua` file, longer than `.lua`, directly inside `.plugins/`); no two
destinations of the whole index collide (`store_dests_collide`: equal, or one is
a folder containing the other).

### `dest` (`plugin_store_dest_valid`, `store_dest_path_valid`)

- 1 to 255 chars, not starting with `/`.
- Characters only `A-Za-z0-9 . _ - /` and space.
- No empty segment, no `.` or `..` segment.
- Not `.compas` or below it.
- Nothing starting with `.plugins/.` (player-owned dot files).
- Not ending in `.upt`.
- No segment ending in `.` or a space. vfat and Windows strip those, so
  `.compas./compas_player` or `r1.upt.` would reach player-owned paths. This is
  deliberately stricter than `plugin_store_dest_valid`; the player is being fixed too.

### Preview (`parse_plugin_preview`, `parse_preview_metadata`)

Metadata object: `asset` token (1 to 127), `sha256` (64 hex), `size` 1 to
65536, `width` 1 to 240, `height` 1 to 400. Optional `variants`: array of at
most 3 more metadata objects. Any invalid object rejects the index. The player
picks the largest variant that fits its screen; the desktop app always uses the
top-level preview. A preview is never installed to the card. The player also
checks the JPEG structure; the desktop app only checks size and SHA-256 and
lets the webview decode it.

## Version compare (`store_version_compare`)

Split at `.`, compare numerically group by group (leading zeros ignored, no
overflow: compare by length of the digit run, then bytewise). A missing group
counts as 0 (`1.2` equals `1.2.0`). Returns -1, 0 or 1.

## Classification

Computed from the destinations of a plugin (index entry or record entry).

- **Theme pack** (`plugin_contains_theme_pack`): some `dest` starts with
  `Themes/`, ends in lowercase `.theme`, and is longer than 13 chars; the part
  after `Themes/` is under 64 chars, contains no `..` and no `\`, has no
  control chars, no segment starting with `.`, no empty segment, and at most 2
  `/`.
- **Player layout** (`plugin_contains_player_layout`): not a theme pack, and
  some `dest` is an `.xml` file (case-insensitive, name longer than `.xml`,
  no further `/`) located directly in `.plugins/`, in
  `.plugins/player_layouts/`, or in `.plugins/<bundle>/player_layouts/`.
- Otherwise it is a plain plugin. Theme packs win over layouts.

## Installed record `.plugins/.store_installed`

Text, `\n` line endings, no escaping (paths cannot hold tabs or newlines):

```
P <id> <version>
F<TAB><dest><TAB><sha256><TAB><keep:0|1>
```

Each `P` line is followed by at least one `F` line. `plugin_store_record_read`
treats the whole file as empty (zero installed plugins) if any rule is broken:

- file and `.plugins/` must be real (non-symlink) regular file and directory;
- lines under 512 chars, each ends with `\n`;
- `P` line: exactly two fields, valid id, valid version, no duplicate id, a
  previous `P` must already have an `F`;
- `F` line before any `P`, bad dest, bad sha, keep not 0 or 1 (read with `strtol`: `+1` and `01` are accepted), more than 32
  files per plugin, or a destination repeated anywhere in the file (case-insensitive);
- file ends with a `P` without `F`.

Limits: at most 400 plugins (`STORE_MAX_RECORD`); a file with more `P` lines is
unreadable. The desktop app reports that as an error and writes nothing, rather
than treating it as empty. New installs of a plugin not yet recorded stop at 400.

### Writing (`record_write_path`)

1. Ensure `.plugins/` exists as a real directory.
2. Write all entries to a unique temporary file in `.plugins/`
   (`.store_installed.tmp.<unix time>.<counter>`, created exclusively, mode 0644).
3. `fflush`, `fsync`, close; on any failure delete the temp file.
4. `rename` over `.store_installed`.

### Record after an install (`write_record_after`)

Start from the old entries. Drop the entry with the same id (it is replaced).
From every other entry remove any file whose destination the new plugin now
owns; drop an entry left with no files. Append the new plugin (its `version`,
all its files with their `sha256` and `keep`) last.

## States (`plugin_store_compute_state`)

Per catalog plugin, given the recorded version `installed` (if any):

- `incompatible` flag = `api_min > plugin_api`.
- Recorded:
  - catalog version `<=` recorded: **installed**;
  - catalog version `>` recorded: **update**, or **installed** if the plugin is
    incompatible.
- Not recorded:
  - the script destination exists on the card as a regular file (`lstat`, not
    a symlink): **manual** (installed outside the store);
  - else **incompatible** if the flag is set, else **available**.

Recorded plugins missing from the catalog are listed as **removed**, with their
recorded version (`compute_results`). Rows carry `player_layout` and
`theme_pack` from the classification above.

When the card's `plugin_api` is unknown (no `device.json`) the desktop app
cannot evaluate the flag and treats every plugin as compatible.

## Install and update (`install_worker`, `plugin_store_commit_local_internal`)

Install and update share one path; only the preconditions differ.

Preconditions, in order:

1. `api_min <= plugin_api`, else "needs a newer player version".
2. Not recorded and 400 entries already: refuse.
3. Install: already recorded and not forced: refuse. Update: catalog version
   must be newer than the recorded one.
4. Free space >= sum of the sizes of the files that will be downloaded + 2 MiB
   (`STORE_MARGIN`). A file with `keep` whose destination already exists is not
   downloaded and not counted.

Staging: a new directory `.compas/store/<id>.<time>.<n>/` on the card. Each
file is downloaded to `<stage>/<i>.part` (i = index in `files`), verified for
size and SHA-256 (`store_download_verified`), at most 1 MiB, redirects allowed.
Any failure deletes the staged files and the directory.

Local-file conflict (`conflict_exists`, `tracked_same`): for each file without
`keep`, if the destination exists it must be a regular file whose SHA-256
equals the one recorded for this same plugin id at that destination. Anything
else (unrecorded file, file edited since install, file owned by another
plugin, non-regular file) is a conflict. Without `force` the operation stops
with "needs confirmation", nothing changed on the card; the caller retries with
`force=true` after the user confirms.

Commit (with the stage already verified):

1. For each file in order:
   - `keep` and destination exists: skip (not touched, not downloaded).
   - destination exists: if not `tracked_same` and not forced, abort as a
     conflict.
2. For each non-skipped file: create parent directories (every component must
   be a real directory, never a symlink); if the destination exists rename it
   to `<stage>/<i>.backup`; rename `<stage>/<i>.part` to the destination.
3. Write the new record (see above). If this fails, roll back.
4. Remove files of the _old_ record of this plugin that are no longer in the
   new file list, are not `keep`, and still have the recorded SHA-256 (a
   modified file is left).
5. Delete the backups, `sync`, remove the stage directory.

Rollback (any failure in steps 2 and 3): in reverse order delete each
destination written and rename its backup back; then delete leftover parts.

`keep` semantics: the file is installed only when absent, is never replaced
or overwritten by updates, is recorded, and is never deleted on uninstall or
when dropped from a newer version.

## Update all (`update_all_worker`)

Walk the catalog; for each plugin that is recorded, has a newer catalog
version and is compatible: if a conflict exists, mark it as needing
confirmation and skip; if free space is short or the download fails, remember
the failure and continue; otherwise stage and commit (non-forced) and reload the
record before the next plugin. At the end: any failure gives a failed result
(the last failure message), otherwise any confirmation gives a needs-confirm
result, otherwise success. Plugins marked for confirmation are then updated one
by one with `force`.

## Uninstall (`uninstall_worker`)

The plugin must be recorded. For each recorded file that is not `keep`: if the
path is a regular file below real directories, rename it into
`<stage>/<i>.backup` (`.compas/store/uninstall-<id>.<time>.<n>/`); a missing file
is ignored; a failed rename restores the earlier backups and aborts. Then write
the record without that entry (`write_record_without`); on failure restore all
backups; on success delete the backups and the stage directory. Empty parent
directories are not removed. The SHA-256 of files is not checked.

## Enable and disable (`plugin_disabled_list_*`)

`.plugins/.disabled`: UTF-8 text, one `.lua` file name (not path, no `.plugins/`
prefix) per line, `\n` terminated; empty lines are ignored and `\r\n` accepted.
At most 64 names; names are case-sensitive and compared as raw bytes with `strcmp`, so non-UTF-8
names are legal and the desktop app keeps every line verbatim when rewriting. Lines
are read up to 255 bytes.

Write (`write_disabled_list`): write the full list to `.plugins/.disabled.tmp`,
`fflush`, `fsync`, close, `rename` over `.disabled`, then best-effort `fsync` of
`.plugins/`. On failure the old file is untouched. Disabling an already
disabled name or enabling an enabled one is a no-op success; disabling beyond 64
names fails. The key of a store plugin is the file name of its script as recorded on the card
(the installed record, not the newer catalog version, which may rename it) (`classify_loaders` maps record entries to loader files the same way).
