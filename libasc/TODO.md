- Add more things that are undoable to the `Action` log

- Have the ability to open some kind of "ticket" for transferring ownership of a commit to another user

- Add hooks for the repository after different events are triggered
  - These will be Python scripts that are mapped to events in the repository, not by directory structure or something similar
  - This means PyO3 will be needed to create bindings to the Rust objects
  - Can be used to enforce things like branch protection

- Add repository-level config and user-level config, customising things like default branch name

- Add content validation in pulling

- Add partial pull/push operations so that repositories are not left in a corrupt state if the operation fails
