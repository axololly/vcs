- Add the ability to create users with permission levels and such

- Add the ability to push and pull from remotes (and therefore sync with remotes)
    - Requires designing a protocol 
    - Also set up servers in a single command like `fossil server`

- Change file structure to be a workspace instead of a project
    - `backend` and `io` in its own crate
        - `remote` in `backend`?
    
    - `commands` in another
