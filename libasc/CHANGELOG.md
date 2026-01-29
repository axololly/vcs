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

- Added user accounts to the repository
- Added project codes to repositories so you can't sync to unrelated repositories
- Added storing edits on snapshots separate to the original snapshot data
- Added support for tilde paths when opening/loading a repository
- Added `Graph::invert` for inverting the directions of a graph
- Added `Graph::extend` to add new connections from a `Graph` to another `Graph`
- Added `Repository::validate_history` to ensure that all hashes are correct and their order is intact
- Added more actions like `Action::MoveBranch`
- Added a pull operation

### Changed

- `Repository::current_user` became `Repository::current_username` and was privatised
- `Repository::current_user` is now a method that retrieves a `Option<&User>` for the current user
    - If the set user is invalid for the operation, `current_username` is reset to `None` and the function returns `None`
- `xdelta3-rs` is now included in the project as Git repositories instead of local folders
- Timestamps on snapshots now use UTC instead of local time
- `Snapshot::from_parts` is renamed to `Snapshot::new`
- `RawContent` was removed and `Content` now directly holds the bytes (a `String` is obtained from `Content::resolve`)
- User accounts generate a public-private key pair instead of using a password
- Merged IO bindings with the rest of the code in `libasc`
- User accounts are stored in a hashmap of public key to `User` object for faster lookups
- `SignedSnapshot` now stores the parents at signage for data integrity (take the merkle pill neo)
- `Snapshot` now inherits the methods and attributes of `SignedSnapshot`
- `Repository::capture_current_state` was replaced by `Repository::commit_current_state`
- `Repository::cwd_differs_from_current` now takes a `&BTreeMap<PathBuf, ObjectHash>` instead of an entire `Snapshot` object
- Saving a repository validates the history first before committing changes to disk
- `Stash` is now a collection type
- Stash entries are now identified by a simple number and store a timestamp of when they occurred
- `Repository::branches` and `Repository::tags` now use a custom type `NamedHashes` over a primitive `HashMap`
- When inserting a hash into a `Graph`, the parent is created as an orphan if it doesn't exist, instead of returning a `Result`
- `Repository::fetch_string_content` now resolves the `Content` before returning
    - `Repository::fetch_content_object` does what the old `Repository::fetch_string_content` did
- `Repository::current_username` uses an `Arc<RwLock>` instead of a `RefCell` for `Send + Sync` capabilities
- `Repository::current_username` holds a `PublicKey` instead of a `String` (usernames can change while keys do not)

### Fixed

- Upgraded cryptographically insecure SHA1 to SHA256
- `Repository::save_string_content` previously saved it as a `&str`, but `Repository::fetch_string_content` would have loaded it as a `Content`
- `Repository::replace_cwd_with_snapshot` did not update `Repository::staged_files`, causing errors when saving due to missing content
- Fixed a really really unsound use of `transmute`

### Removed

- Removed `Action::RebaseSnapshot`
