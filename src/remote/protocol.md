## Overview

1. send user creds and get permission from remote
2. instruct local and remote to construct a minimised DAG from the branch tips to the root
3. use RIBLT on the minimised DAG to get a list of commits to send
4. send payload to remote
5. remote validates content (eg. checking hashes are correct)
6. with the sent commits, a second RIBLT is used to decide which content needs to be sent to the remote
7. remote validates this content too (eg. checking hashes are correct)
8. finally, tags are sent over -  conflicting tags send back an error but the content is kept for a reattempt
9. remote updates its tags, reports success, and closes connection

## Authentication

- Exchange user credentials to the remote
    - Absence of user credentials results in the `anonymous` user (like Fossil)
    - Permissions will be configured in a TOML file under `.asc/users/perms.toml`
    - Passwords will be stored in MessagePack under `.asc/users/<username>`
- Depending on the operation, accept or reject the user

<!-- Include stuff about exchanging and then fetching missing artifacts with RIBLT -->