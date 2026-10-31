# Notes for Claude

Read by every Claude Code session in this repository. The maintainer writes in German; answer in
German and address them informally ("du"), keep code, commits and docs in English.

## Before starting any work

Several sessions work on this repository, often on the same day. Before writing code, look at
what already exists, so no work is done twice:

1. Read "Current state" below and the roadmap item the work belongs to.
2. `git fetch origin` and list the remote branches (`git branch -r`) and the open pull requests
   (GitHub tools). A branch or pull request for the same roadmap item means: continue there,
   don't start anew.
3. Read the code the change touches before planning it.

## What cloud sessions can't do

- **Delete remote branches or force-push**: the cloud environment (claude.ai/code) refuses
  `git push --delete` and `git push --force-with-lease` as destructive. So: never push something
  that would need to be undone that way, and use a new commit instead of amending a pushed one.
  A branch that has to go is deleted by the maintainer on GitHub, or by a Claude Desktop session
  on the Mac, which can probably do it (not tried yet).

## Current state

Handover between sessions; the session that changes the state updates this section. New sessions
start from `main` and only see what is there, so the update goes to `main` in its own small
`docs/` pull request (merged right away), not only in the feature branch. As of 2026-10-31:

- **M1a, task manager** (see the roadmap):
  1. Core: merged (#4).
  2. Import as tasks: merged (#5), tested on the Mac 2026-10-31. Disc named in the task title
     of a game with several discs: merged (#15), confirmed on the Mac.
  3. Convert, export, scrub and checksums as tasks, `LongOperations` removed, reordering: merged
     (#10) before the test on the Mac. Partly tested 2026-10-31: queue, "Earlier", cancel, hash
     work; export, convert and scrub still need the maintainer (item 3 in "Waiting for the
     Mac"). A paused task shows no speed and counts as paused: merged (#27).
  4. Stay awake: pull request **#7**, branch `claude/m1a-stay-awake`, open; held while
     `tasks.is_busy()`. Tried on the Mac 2026-10-31: `caffeinate` runs only while a task runs.
     The long sleep test is still due (on a Mac without Amphetamine or other sleep blockers).
  5. Unfinished tasks after a restart: pull request **#13**, branch `claude/m1a-resume-tasks`,
     open. Tried on the Mac 2026-10-31: works, with the fix for the missing Tasks icon after a
     restart (21a8aec1); cleaning up leftovers on a real drive not tried. #11 and #13 each add a "Waiting for the Mac" item 5 and
     touch `jobs.rs`: the one merged second renumbers its item and adds `SavedJob::Install`.
- **M1, install on the 3DS**: pull request **#11**, branch `claude/m1-3ds-install-and-consoles`,
  open: "Install" on a CIA as a task (`Job::Install`, `Resource::Console`), consoles found in
  the background every 15 seconds (failures only logged), installed titles remembered in
  `consoles.json`. Ported from the local branch `feature/task-manager`, which predates #4 and
  has its own task manager; nothing else there is needed (its M1d roadmap is #8, staying awake
  is #7). `main` merged in 2026-10-31. To check on the 3DS (see the Mac checklist): pausing an install holds
  the TCP connection open, which Link may give up on (then Install tasks shouldn't offer Pause);
  cancelling drops the connection, so check that no half-installed title is left. Waits for the Mac test (item 5 in "Waiting for the Mac" on that branch) before merging.
- **M0b**: done (#18, tested on the Mac 2026-10-31).
- A drive that doesn't answer no longer holds up the settings: merged (#28). Trying a build on
  macOS: BUILDING.md (#29). New milestone M2b, explain and repair bad dumps (#26): WBFS dumps
  never match the disc's hash.
- A ROM source that doesn't answer (e.g. a hung disk) no longer holds up the library scan: each
  source on its own thread, given up after 15 s without data, keeping its items from
  `library.json`, with a warning: merged (#31), tried on the Mac with gamestation hung.
- A disc already on the drive isn't imported again, from the dialog or by drag and drop: merged
  (#33). This keeps #13's restart cleanup (it deletes every file with the game's prefix) from
  removing a game that was there before. Open: picking the same game twice before its import
  starts still queues two imports: fixed in pull request **#36** (`claude/no-double-import`),
  open, not tried on the Mac.
- A drive that doesn't answer (a hung disk, or macOS's access prompt for removable volumes, which
  every re-signed test build triggers again) emptied the drive list in My Consoles: pull request
  **#35** (`claude/drives-not-answering`), open, tried on the Mac.
- **M2b, explain bad dumps**: first step, pull request **#38** (`claude/m2b-explain-mismatch`),
  open: "Hash mismatch" says when the update partition is missing (32 of the 48 Wii WBFS on
  WiiGames). nod's H0–H3 validation fails on WBFS as it is (see the roadmap).
- **M1a+, notifications and log**: first step, pull request **#20**, branch
  `claude/m1a-plus-group-notifications`, open: the same message again is counted ("245×") in
  one toast; works on the Mac. Second step, pull request **#22**, branch `claude/m1a-plus-system-notifications`,
  open: system notifications (`notify-rust`) for a task done or failed and an update ready while
  the window is in the background, with a setting. Fixed 2026-10-31: an app opened in the
  background never sent any (it assumed the focus). The banner is still not seen; a test build
  may first need the notification permission; #20 and #22 (and #7, a field in `AppState`) touch the same lines,
  the one merged later keeps both. Third step, pull request **#24**, branch
  `claude/m1a-plus-notification-center`, stacked on #20 (merge #20 first): a Messages page with
  every message since the start, a filter for warnings and errors, a search field (added
  2026-10-31, not tried on the Mac), "Open log file"; works on the
  Mac. Test log entries for these Mac tests: drafts in the M0b session's notes. Still open
  for M1a+: see the roadmap.
- Branches to delete on GitHub: `feature/task-manager` once #11 is merged. Sessions on the
  maintainer's Mac can delete remote branches (done for the earlier ones on 2026-10-31).

## Working with the maintainer

- **Recommend a new session proactively** as soon as it makes sense, without being asked: when
  the conversation has grown long, when the next piece of work doesn't need what this session
  knows (a new milestone, an unrelated topic), or when a fresh start is cheaper than carrying the
  context. Say so in one line, with what the new session should start with (branch, pull request,
  roadmap item).
- The maintainer tests on a Mac with real consoles and drives; cloud sessions can't. What waits
  for the Mac is listed in [docs/ROADMAP.md](docs/ROADMAP.md) under "Waiting for the Mac"; add
  to it whenever a change needs a real-hardware test.

## Where things are

- Plan and milestones: [docs/ROADMAP.md](docs/ROADMAP.md); tests on real hardware:
  [docs/TESTLOG.md](docs/TESTLOG.md).
- Branches, commits, pull requests: [docs/GIT_WORKFLOW.md](docs/GIT_WORKFLOW.md). Checks before
  pushing, as CI runs them: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
  warnings`, `cargo test --locked` (needs Rust 1.98 or newer: `rustup toolchain install stable`).
- AI rules: [AI_POLICY.md](AI_POLICY.md).
