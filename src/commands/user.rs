use std::time::Duration;

use clap::Subcommand;
use eyre::{bail, Result};

use crate::backend::{repository::Repository, user::{self, Users}};

#[derive(Subcommand)]
pub enum Subcommands {
    /// Create a new user in the repository.
    /// By default, they will inherit the permissions
    /// of the `everyone` user.
    #[command(visible_alias = "new")]
    Create {
        username: String,
        permissions: Option<String>
    },

    /// Update a user's password in the repository.
    Update {
        username: String
    },

    /// Delete a user from the repository.
    #[command(visible_aliases = ["remove", "rm"])]
    Delete {
        username: String
    },

    /// List all users in the repository.
    List,

    /// Get or set which user the repository is using for commits.
    Current {
        username: Option<String>
    },

    /// Get or set the permissions of a user in the repository.
    #[command(visible_alias = "perms")]
    Permissions {
        username: String,
        permissions: Option<String>
    }
}

pub fn read_stdin_hidden(prompt: &str) -> Result<String> {
    use manyterm::{event::{Event, read}, sys, types::Key, print};

    sys::init()?;

    sys::enable_raw_mode()?;

    print!("{prompt}");

    let mut password = String::new();

    loop {
        let Some(Event::Key(key)) = read(Duration::from_secs(3)) else { continue };

        if key.modifiers.ctrl || key.modifiers.alt {
            continue;
        }

        match key.key {
            Key::Char(ch) => {
                password.push(ch);
            },

            Key::Backspace => {
                password.pop();
            }

            Key::Enter => break,

            _ => continue
        }
    }

    sys::disable_raw_mode()?;

    Ok(password)
}

fn create_user(username: &str, users: &mut Users) -> Result<()> {
    let password = read_stdin_hidden("Enter password: ")?;
    let password2 = read_stdin_hidden("Repeat password: ")?;

    if password != password2 {
        println!("Passwords did not match.");

        return Ok(());
    }

    users.create_user(&username, &password)
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    use Subcommands::*;
    
    match subcommand {
        Create { username, permissions } => {
            if repo.users.get_user(&username).is_some() {
                bail!("the user {username:?} already exists in the repository.");
            }

            let perms = match permissions {
                Some(raw) => user::Permissions::try_from(raw)?,
                None => user::Permissions::empty()
            };

            create_user(&username, &mut repo.users)?;

            let user = repo.users.get_user_mut(&username).unwrap();

            user.permissions = perms;

            println!("Successfully created user {username:?}.");
        },

        Update { username } => {
            if repo.users.get_user(&username).is_none() {
                bail!("the user {username:?} does not exist in the repository.");
            }

            create_user(&username, &mut repo.users)?;

            println!("Updated password of user {username:?}.");
        },

        Delete { username } => {
            if repo.users.remove_user(&username).is_some() {
                println!("Removed {username:?} from the repository.");
            }
            else {
                bail!("the user {username:?} does not exist in the repository.");
            }
        },

        List => {
            if repo.users.is_empty() {
                println!("No users in the repository.");
                
                return Ok(());
            }

            println!("Users:");

            for user in repo.users.iter() {
                println!(" * {}", user.name);
            }
        },

        Current { username } => {
            let Some(name) = username else {
                println!("{}", repo.current_user().name);

                return Ok(());
            };

            if let Some(user) = repo.users.get_user(&name) {
                repo.current_username = user.name.clone();
            }
            else {
                bail!("the user {name:?} does not exist in the repository.");
            }
        },

        Permissions { username, permissions } => {
            let Some(perms) = permissions else {
                println!("{}", repo.current_user().permissions.to_string_pretty());

                return Ok(());
            };

            let new_perms = user::Permissions::try_from(perms)?;

            let Some(user) = repo.users.get_user_mut(&username) else {
                bail!("the user {username:?} does not exist in the repository.");
            };

            let original_perms = user.permissions.clone();

            user.permissions = new_perms;

            println!(
                "Changed permissions of {username:?}: {} -> {}",
                original_perms.to_string_pretty(),
                new_perms.to_string_pretty()
            );
        }
    }

    repo.save()?;

    Ok(())
}