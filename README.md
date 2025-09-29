# 📦 Stash – Smart File Organizer

Stash is a fast, async Rust-powered file organizer.
It scans a target directory, classifies files by type, and moves them into a structured `Organized/` folder.
It also supports **dry runs** (simulation mode) and **revert** (undo last changes).

---

## ✨ Features

* 🚀 **Asynchronous + concurrent file handling** with Tokio
* 📂 Classifies files into categories (e.g., `Documents`, `Images`, `Videos`, etc.)
* 🔒 Avoids duplicate conflicts using hashing + conflict resolver
* 🗄️ SQLite database keeps track of moves (with in-memory mode for `--dry-run`)
* 🔄 **Revert** support to undo the last organize operation
* 🧪 **Dry-run mode** to preview changes before applying

---

## ⚙️ Installation

Clone and build:

```bash
git clone https://github.com/chineduCoded/file_organizer.git
cd file_organizer
cargo build --release
```

The binary will be available at `target/release/stash`.
You can move it into your `$PATH` for convenience:

```bash
cp target/release/stash ~/.local/bin/
```

---

## 🚀 Usage

### Organize files

Organize files inside a directory (e.g., `Downloads/`):

```bash
stash organize ~/Downloads
```

This will create:

```
~/Downloads/Organized/
 ├── Documents/
 │   └── Pdf/
 │       └── 2025/
 │           └── 01StableMatching.pdf
 ├── Images/
 ├── Music/
 └── Videos/
```

---

### Dry run (no changes)

Preview what would happen without actually moving files:

```bash
stash organize ~/Downloads --dry-run
```

Example output:

```
Would move "/home/chinedum/Downloads/01StableMatching.pdf" (category: Documents::Pdf) → "/home/chinedum/Downloads/Organized/Documents/Pdf/2025/01StableMatching.pdf"
✅ Dry-run completed: 51 files analyzed, 51 planned moves
```

---

### Revert

Undo the last `organize` operation:

```bash
stash revert ~/Downloads
```

This will move files back to their original locations based on the database record.

---

### Options

| Command                    | Description                            |
| -------------------------- | -------------------------------------- |
| `organize <DIR>`           | Organize files inside `<DIR>`          |
| `organize <DIR> --dry-run` | Simulate organize without moving files |
| `revert <DIR>`             | Undo last organize for `<DIR>`         |

---

## 📝 Notes

* **Only top-level files** inside the target directory are organized; subdirectories are ignored.
* Already organized files are skipped unless they change.
* The database is stored under `~/.local/share/file_organizer/` by default.
* Dry-runs use an in-memory database.

---

## 📌 Example Workflow

```bash
# Preview changes
stash organize ~/Downloads --dry-run

# Organize for real
stash organize ~/Downloads

# Undo changes if needed
stash revert ~/Downloads
```