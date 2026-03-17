- Have the ability to open some kind of "ticket" for transferring ownership of a commit to another user

- Add hooks for the repository after different events are triggered
  - These will be Python scripts that are mapped to events in the repository, not by directory structure or something similar
  - This means PyO3 will be needed to create bindings to the Rust objects
  - Can be used to enforce things like branch protection

- Add repository-level config and user-level config, customising things like default branch name

- Add bare repositories, probably by equating `main_dir` and `root_dir`

- Add support for deleting branches on remotes

- Add more support for merging, like tracking which files were affected by a merge
  - Could be done with a special file in the .asc/ directory?
