# FSearch Crossplatform

A cross-platform fork of FSearch, whole-disk file search for macOS. Finds
any file by name in about a millisecond, forgives typos, and searches inside
files with an index. Use it as a CLI (with a small daemon) or as a Rust crate.

```
cargo build --release && ./target/release/fsearch install   # -> ~/.local/bin/fsearch
fsearch fsearch main              # find files by name
fsearch 'ext:rs grep:apply_dir'   # search inside files
```

## Speed

M4 Max, 7.7M files and folders on disk.

| | |
|---|---|
| find a file by name, whole disk | p50 1.3 ms |
| search inside files | p50 9 ms |
| a new, renamed or deleted file shows up | ~0.1 s |
| first crawl of the disk | ~20 s, once |
| daemon memory | 30-135 MB |

## vs fff

Chromium (509k files), same Mac, same queries. Video:
[`demo/fsearch-vs-fff.mp4`](demo/fsearch-vs-fff.mp4), method:
[`demo/vs_fff.py`](demo/vs_fff.py).

| | fsearch | [fff](https://github.com/dmtrKovalenko/fff) |
|---|---|---|
| find a file by name | 1.1 ms | 13.8 ms |
| search inside files | 5.6 ms | 53 ms |
| typo still finds the file first | 98% | 88% |
| ready after launch | 50 ms | 2.5 s |
| memory | 50 MB (whole disk) | 358 MB (that folder) |

On the smaller Linux kernel (96k files), name search is a tie and fsearch
wins the rest. fff searches the contents of about 9% more files, because
fsearch skips some file types and `build/` and `vendor/` folders.

## Queries

```
fsearch 'readme in:~/Developer'          # inside a folder
fsearch 'type:image size:>5mb mtime:<7d'
fsearch 'ext:rs regex:fn\s+\w+_dir'      # regex inside files
fsearch 'sym:apply_dir'                  # where it's defined
```

Words are fuzzy, and 5+ letter words forgive one typo (`mian.rs` finds
`main.rs`). Also `'exact`, `^prefix`, `suffix$` and `!exclude`. Filters:
`ext:` `type:` `kind:` `in:` `size:` `mtime:` `re:` `path:` `grep:` `regex:`
`sym:` `limit:`. Content search is smart-case.

## Full Disk Access

Started from a terminal with Full Disk Access, it indexes everything. As a
login item (`fsearch install --login`), give `~/.local/bin/fsearch` its own
grant in System Settings > Privacy & Security, again after each rebuild.
Without access it skips the protected folders instead of popping a prompt.

## Goals of this Fork

- No change in behavior or logic on macOS compared to upstream.
- Support Linux and other platforms with parity to upstream where possible.
- Drop-in support as a dep for apps already using upstream with the same
binary-name (`fsearch`) as upstream.
- Staying up to date with the direction taken upstream.

## Non-Goals

- Adding GUI, TUI, or other features not present upstream beyond what's
needed for cross-platform parity.
- Bugfixes in platform-shared code before upstream gets them.

## Linux

Indexes `$HOME` with `getdents64` and watches it with inotify. State lives in
`$XDG_DATA_HOME/fsearch`, else `~/.local/share/fsearch`. `fsearch install
--login` installs a systemd user unit. There is no event replay. Restarts
recover with a `synced_at` mtime relist.

### Limits

Each watched directory costs one inotify watch, so very large trees
can exceed `max_user_watches` (a developer home here needs ~465k against
Arch's 524k default; lower-default systems will hit the wall, so to fix you
can raise that and restart the daemon). Bursts of files in brand-new directories
can outrun watch establishment and surface on the next change or restart.
Cross-device mounts under home (like a flash drive mounted to the path `~/external`)
are skipped by the scan.

## API

JSON lines over `fsearch.sock` in the data dir, or `fsearch stdio`:

```json
{"q": "fsearch main", "limit": 20}
{"op": "grep", "pattern": "apply_dir", "in": "~/Developer"}
```

Or link the crate:

```rust
let engine = fsearch::Engine::start(fsearch::Options { dir: fsearch::default_dir(&home), home: home.clone(), skip: None })?;
let hits = engine.search(&fsearch::Query::parse("fsearch main", &home)?)?;
```

An app and the CLI share one index: the first process owns it and the
others follow along.

## How it works

- Crawls the disk once with `getattrlistbulk`, then stays current from
  FSEvents. A restart replays only what changed.
- Names live in one mmap'd file, laid out folder by folder so `in:` is a
  range. Each distinct name is scored once.
- Content search uses a trigram index of your text files. Matches are read
  fresh from disk, so they're never stale.
