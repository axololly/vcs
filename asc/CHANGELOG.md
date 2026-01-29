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

### Removed

- `asc rebase` is removed until further notice (it's cringe and `merge` is more based)

### Fixed

- `asc update` didn't refill `Repository::staged_files`
- `asc branch` didn't update `Repository::action_history`
- Fixed display of `asc log`
- Previously used SHA1 while `libasc` used SHA2
