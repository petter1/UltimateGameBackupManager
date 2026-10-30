# Notes for Claude

Read by every Claude Code session in this repository. The maintainer writes in German; answer in
German, keep code, commits and docs in English.

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

Handover between sessions; the session that changes the state updates this section (in its pull
request). As of 2026-10-30:

- **M1a, task manager** (see the roadmap):
  1. Core: merged (#4).
  2. Import as tasks: pull request **#5**, branch `claude/m1a-import-tasks`; `main` merged in,
     checks green, waits for the maintainer's review and a test on the Mac (see "Waiting for the
     Mac" in the roadmap). Has pause, resume, retry, cancel and "Cancel all" already.
  3. Convert, export, scrub and checksums as tasks, remove `LongOperations`, task manager page
     with reordering and unfinished tasks after a restart: **not started**. The next piece of
     work; start it from `claude/m1a-import-tasks` (or from `main` once #5 is merged).
  4. Stay awake: pull request **#7**, branch `claude/m1a-stay-awake`, open.
- **M0b**: only the Mac part is left (`.icns`, screenshot).
- A second, parallel version of #5 was pushed as `claude/m1a-import-tasks-alt` by mistake; the
  maintainer deletes it on GitHub (sessions may not delete remote branches). Its one useful
  change (reload the games only when the import went to the drive shown) is in #5.

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
