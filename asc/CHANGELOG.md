# Changelog

This is where all the changes to this crate will be going.

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

- The ability to move branches with `asc branch mv` (does the equivalent of deleting and recreating the branch)
- `asc remote` lets you interact with remote URLs in the repository
- `asc history` can filter for snapshots that change a file, or are before/after a given datetime
- Added commands for cloning, pushing and pulling
    - These also list how many bytes were sent and received
- Added viewing content blobs as well as snapshots with `asc show`
- `asc history` lists potential branches and tags on commits
- `asc show` now lists any tags or branches that the commit is on, and the hash of content blobs
- `asc ls` can now include changes on the files (`-v` is for version, `-c` is for changes)

### Changed

- Changed all `bail!` calls and some `unwrap!` calls to use `eprintln!` instead
- `asc branch delete` can now take multiple names and a `--keep-going` flag
- Commands now use bold bright green text instead of basic green text

### Removed

- `asc rebase` is removed until further notice (it's cringe and `merge` is more based)
- Permissions are not displayed on user accounts

### Fixed

- `asc update` didn't refill `Repository::staged_files`
- `asc branch` didn't update `Repository::action_history`
- Fixed display of `asc log`
- Previously used SHA1 while `libasc` used SHA2
- `asc modify` rehashes its children like Git does
- `asc pull` and `asc push` were repeating what `libasc` was doing (moving branches, logging actions, etc), causing the branch pointers to be updated incorrectly
- `asc add`, `asc remove` and `asc update` were handling globs incorrectly
- `asc stash new` was overwriting the stash due to recursion
- `asc add` was interacting with the terminal incorrectly
- `asc cat` was adding an extra newline due to using `println!`
- `asc clean` no longer deletes root commits, tagged commits or the currently referenced commit
- `asc diff` previously didn't have `from` and `to` as labelled arguments
- `asc ls`, `asc mv` and `asc rm` now use `filter_paths_with_glob_strict` instead of `filter_with_glob` or alternate logic
