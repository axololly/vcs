# Changelog

This is where all the changes to this project will be going.

For my own personal reference:

- **MAJOR** (1.0.0 → 2.0.0): Breaking changes
- **MINOR** (1.0.0 → 1.1.0): New features (backwards compatible)
- **PATCH** (1.0.0 → 1.0.1): Bug fixes (backwards compatible)

Categories are as follows:

- `Added` for new features
- `Changed` for changes in existing functionality
- `Deprecated` for soon-to-be-removed features
- `Removed` for now removed features
- `Fixed` for any bug fixes
- `Security` in case of vulnerabilities

## \[Unreleased\]

### Added

- `FileChange` enum to represent a change in a file (added to the repo, removed from the repo, missing from the repo, etc)
- `asc undo` and `asc redo` now have `--all` to undo/redo all actions on the repository.

### Changed

- `Repository::create_new` simply creates a `Repository` object from scratch, then calls `.save()` on it

### Fixed

- Fixed an issue where tags were not saved to disk because I was stupid enough to leave in code that saves the trash to disk twice


## v0.5.0

### Added

- `Snapshot::from_parts` to construct a `Snapshot` the same way consistently across the codebase

### Changed

- `Snapshot` has changed to use a `BTreeMap` to preserve order for hashing

### Fixed

- Documentation on `Repository::capture_current_state` was left unfinished
- `Repository::capture_current_state` now hashes the author, message and timestamp
- `asc modify` also updates the hash

### Removed

- `Action::CreateSnapshot` and `Action::DeleteSnapshot` no longer exist as `asc trash` replaces the need for these


## v0.4.0

### Added

- Documented everything in the repository
- `ObjectHash::as_bytes` yields the internal bytes of the hash
- `asc blame` that functions like `git blame`
- Added `Graph::is_descendant`

### Changed

- Changed some files to use the `unwrap!` macro instead of `.ok_or(eyre!(...))?`
- Privatised `Graph::links`
- Removed unnecessary loop in `Graph::remove`
- Replaced `Repository::cwd_differs_from_current` with `Repository::has_unsaved_changes`
- Replaced `Repository::snapshot_from_paths` with `Repository::capture_current_state`

### Fixed

- `asc trash` now saves its changes to disk
- `Repository::has_unsaved_changes` also checks stashed snapshots, which `Repository::cwd_differs_from_current` did not do
- `Repository::create_new` now asserts that the directory exists
- Some files would not be created by `Repository::create_new`, which would've broken commands
- `asc history`'s `--limit` argument was previously ignored

### Removed

- `Change<T>` in `src/backend/action.rs` was unused
- `ObjectHash` no longer implements `Deref` and `DerefMut`
- Removed `Stash::staged_files` because it can be accessed on the stash's snapshot's `files` attribute
- `Repository::snapshot_from_paths` was removed for not saving the snapshot to disk


## v0.3.0

This isn't a full list because I decided to not make incremental commits and instead do one giant commit with each version. Horrible idea, but here you go.

### Added

- Added `asc cat` and `asc ls` to list directory contents and view files
- Added an undo-redo stack accessible through `asc undo`, `asc redo` and `asc log`
- Added a way to clean up unreferenced commits with `asc clean`
- Added `DisplaySeq` for printing a sequence like `Vec` with every item using `Display` instead of `Debug`
- Added `Repository::stashes` and `asc stash` for stashing states
- Added `asc trash` for moving commits to the trash
- Added `asc update` that operates the same as `fossil addremove`
- Added `asc modify` to modify a commit in-place, but not changing its hash
- Added `Repository::replace_cwd_with_snapshot` to implement the behaviour of `asc switch` and `asc stash apply|pop`
- Added merging two branches with `asc merge`
    - Merge conflicts operate like Git where the valid files are staged but the conflicting files are unstaged

### Changed

- `Link` now uses `BTreeSet` instead of `Vec` for storing links
- `Display` for `CommitHash` outputs the hash shrunk to 10 characters
    - The full hash is accessible through `Debug` or `CommitHash::full`
- `CommitHash` is now `ObjectHash` to be generic across commits and stashes
- The raw `ObjectHash` is dumped and loaded directly, instead of through its string representation
- The `editor` argument to `asc commit` is not positional anymore
- `Commit` has become `Snapshot` to be generic across commits and stashes
- `Snapshot`'s loading and dumping functionality is now the responsibility of `Repository`
- `Snapshot::from_paths` has been replaced by `Repository::snapshot_from_paths`
- `Tree` is now `Graph` because the data structure is not technically a tree
- `Snapshot` now holds a "pointer" to each file's content to reduce memory overhead, which is now stored as a separate immutable object
- `Snapshot` is loaded and dumped directly
- `ProjectInfo` is loaded and dumped directly
- `Repository::snapshot_history` has been replaced with `Repository::history`
- `asc clean` also clears out the trash bin

### Removed

- `thiserror` is not used anymore in favour of `eyre!`
- Removed IO functionality for `Snapshot`
- Removed `SnapshotHeader` (originally `CommitHeader`)
- Removed unnecessary conversion of `PathBuf <-> String` when saving a repository
- `Graph` no longer holds the children of each snapshot

### Fixed

- Code previously didn't compile because of declaring a module that didn't exist


## v0.2.0

### Added

- Added quite a lot of commands for interacting with the backend through a CLI:
    - `asc add` and `asc remove` for staging files
    - `asc commit` for making commits
        - This now opens an editor (specified by the `EDITOR` environment variable) if the command line message is omitted
        - Currently, empty messages are disallowed. Why? They're cringe.
    - `asc history` for viewing a commit history
    - `asc branch` for interacting with branches
        - This now has several subcommands and functions like Fossil
        - `asc branch list` lists branches like Git does, with colour
    - `asc switch` for changing between versions (go to a branch or a commit hash)

- Added creating an `.ascignore` file when initalising a repository

### Changed

- Changed name from `vcs` to `asc`

- Changed `Repository::root_dir` to point to the directory containing the `.asc` directory instead of the `.asc` directory itself

### Fixed

- Fixed being able to make commits where there were no changes

### Removed

- Removed `Repository::switch_branch` because `asc switch` can also take a commit hash


## v0.1.0

### Added

- Added various files in `backend/`:
    - `commit.rs` for representing commits
    - `hash.rs` for a dedicated commit hash type with many interchangeable formats
    - `repository.rs` with a simple API for loading, saving and modifying a repository
    - `tree.rs` with support for linking commits

- Added corresponding IO bindings for interactions with disk
    - By separating core functionality from IO, hopefully developing this will be easier.