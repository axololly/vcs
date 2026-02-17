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

### Changed

- Changed all `bail!` calls and some `unwrap!` calls to use `eprintln!` instead

### Removed

- `asc rebase` is removed until further notice (it's cringe and `merge` is more based)
- Permissions are not displayed on user accounts

### Fixed

- `asc update` didn't refill `Repository::staged_files`
- `asc branch` didn't update `Repository::action_history`
- Fixed display of `asc log`
- Previously used SHA1 while `libasc` used SHA2
- `asc modify` rehashes its children like Git does
