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