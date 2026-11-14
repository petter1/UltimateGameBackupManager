# UnRAR linking exception (and required notice)

Ultimate Game Backup Manager is licensed under **GPL-3.0-only** (see `../COPYING`); much of it is
TinyWiiBackupManager's code by Manuel Quarneti. Its `.rar` support is built on the **UnRAR**
source code, compiled (via the `unrar` / `unrar_sys` crates) into one program of its own,
**`ugbm-unrar`** (`crates/ugbm-unrar`). UnRAR is **only used to *extract* RAR archives** — the
app does not, and the UnRAR code here may not, be used to compress RAR or build a RAR-compatible
archiver.

`ugbm-unrar` is taken over from ps5upload's `ps5upload-unrar`
(<https://github.com/phantomptr/ps5upload>, `engine/crates/ps5upload-unrar`), and this exception
from ps5upload's `LICENSES/UnRAR-exception.md`. Thanks to phantomptr and the ps5upload
contributors. `ugbm-unrar` contains **only ps5upload's code, this project's own code, plus
UnRAR** (and the permissively licensed Rust crates it needs: `unrar`, `serde`, `serde_json`),
and none of TinyWiiBackupManager's. It is a separate executable: the app does **not** link
UnRAR; it starts `ugbm-unrar` and exchanges requests and decoded bytes with it over pipes. The
app contains GPL code whose authors grant no UnRAR exception, which is why UnRAR must stay out of
that binary. The release packages ship `ugbm-unrar` next to the app.

UnRAR's own license is reproduced verbatim in `UnRAR-license.txt`. It is compatible with
permissive use but adds a field-of-use restriction that the GPL would otherwise forbid combining
with GPL code (GPLv3 §7). To resolve that, the code in `crates/ugbm-unrar` is licensed under
GPL-3.0-or-later with the following additional permission: ps5upload's copyright holders grant it
for their code (and allow it to be extended to versions of it), and the copyright holders of this
project grant it for their changes.

## Additional permission under GNU GPL version 3 section 7

> As a special exception, the copyright holders of ps5upload and of Ultimate Game Backup Manager
> give you permission to combine `ugbm-unrar` with the UnRAR source code (and the `unrar` /
> `unrar_sys` Rust crates that bundle it), and to convey the resulting work. You may extend this
> exception to your version of `ugbm-unrar`, but you are not obligated to do so. If you do not
> wish to do so, delete this exception statement from your version.

## Required UnRAR notice

> UnRAR source code may be used in any software to handle RAR archives without limitations free
> of charge, but cannot be used to develop RAR (WinRAR) compatible archiver and to re-create RAR
> compression algorithm, which is proprietary. Distribution of modified UnRAR source code in
> separate form or as a part of other software is permitted, provided that full text of this
> paragraph, starting from "UnRAR source code" words, is included in license, or in documentation
> if license is not available, and in source code comments of resulting package.
