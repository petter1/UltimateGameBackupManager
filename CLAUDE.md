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
`docs/` pull request (merged right away), not only in the feature branch. As of 2026-10-31 (afternoon):

- **Merged 2026-10-31 after review** (all tried on the Mac before): #20 (the same message
  counted, "245×"), #24 (Messages page; its search not tried on the Mac), #7 (stay awake while
  `tasks.is_busy()`; the long sleep test is still due), #35 (a drive that doesn't answer stays in
  the list as "Doesn't answer"; since the review, a drive whose earlier read still hangs isn't read
  again), #38 (M2b first step: "Hash mismatch" says when the update partition is missing), #40
  (test log of 2026-10-31).
- **M1a, task manager**: core (#4), import (#5, #15), convert/export/scrub/hash (#10, #27)
  merged. Export, convert, scrub and pause by hand still need the maintainer (item 3 in "Waiting
  for the Mac"). Unfinished tasks after a restart: pull request **#13** (`claude/m1a-resume-tasks`),
  open. Worked on the Mac, but the review changed a lot that touches files on the drive (an import
  checks again when it starts that its disc isn't there yet; the cleanup keeps the files when the
  source was removed; an unrestorable task stays listed; a damaged `tasks.json` entry is skipped
  with a warning): try it again on the Mac before merging. #11 and #13 both touch `jobs.rs`: the
  one merged second adds `SavedJob::Install` (`to_saved`, `leftovers`, `restore`) and renumbers
  its "Waiting for the Mac" item 5.
- **M1, install on the 3DS**: pull request **#11** (`claude/m1-3ds-install-and-consoles`), open,
  not reviewed again yet: "Install" on a CIA as a task, consoles found every 15 seconds,
  installed titles in `consoles.json`. To check on the 3DS: pausing an install holds the TCP
  connection (Link may give up; then Install tasks shouldn't offer Pause); cancelling must not
  leave a half-installed title.
- **M1a+, notifications**: system notifications, pull request **#22**
  (`claude/m1a-plus-system-notifications`), open, reviewed. The banner hasn't been seen on the Mac;
  likely reasons from the review: notify-rust sends on macOS when the handle is dropped and ignores
  errors there; mac-notification-sys falls back to `com.apple.Terminal` when the bundle id isn't
  registered with Launch Services; the old `NSUserNotificationCenter` API (newer one: notify-rust
  feature `preview-macos-un`, needs permission and a signed bundle). Still open for M1a+: see the
  roadmap.
- **Imports**: picking the same game twice before its import starts no longer queues two, and
  Retry refuses a game already being imported: pull request **#36** (`claude/no-double-import`),
  open, reviewed, not tried on the Mac.
- **M1c, copy many**: "Copy all to drive", pull request **#41** (`claude/m1c-copy-many`, M0b
  session), open; review findings fixed (free space, read-only drive, unreadable files, game list
  not loaded). Relies on #36 against queueing a game twice: merge #36 first. The click that writes
  to WiiGames is untested.
- **Next on the Mac, in this order**: #36, #41, #13 (again, after the review's changes), #22 (the
  banner), #11 (3DS); then the long sleep test for #7 and item 3 of "Waiting for the Mac". Each
  one that works may be merged by Claude when the maintainer says so.
- **M2b, explain and repair bad dumps** (#26): first step merged (#38). nod's H0–H3 validation
  fails on WBFS as it is (see the roadmap). Open: only the first disc of a multi-disc game is
  hashed.
- A ROM source or drive that doesn't answer no longer holds up the library scan (#31) or the
  settings (#28). Trying a build on macOS: BUILDING.md (#29); every re-signed test build asks again
  for removable volumes access.
- Branches to delete on GitHub: `feature/task-manager` once #11 is merged.

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
