use clap::Subcommand;
use color_eyre::owo_colors::OwoColorize;
use eyre::{bail, Result};

use libasc::{repository::Repository, unwrap};

// TODO: include more subcommands for things like closing and reopening user accounts
#[derive(Subcommand)]
pub enum Subcommands {
    /// Create a new user in the repository.
    /// By default, they will inherit the permissions
    /// of the `everyone` user.
    #[command(visible_alias = "new")]
    Create {
        username: String
    },

    /// List all users in the repository.
    #[command(visible_alias = "ls")]
    List,

    /// Get or set which user the repository is using for commits.
    Current {
        username: Option<String>
    },

    /// List information about the given user.
    Info {
        username: Option<String>,

        /// Whether or not to show the private key.
        #[arg(long)]
        show_private_key: bool
    },

    /// Close a user account, making it unusable for authentication.
    Close {
        username: String
    },

    /// Reopen a user account, making it usable for authentication.
    Reopen {
        username: String
    },

    /// Rename a user account.
    Rename {
        old: String,
        new: String
    }
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    use Subcommands::*;
    
    match subcommand {
        Create { username } => {
            let user = repo.users.create_user(username)?;

            println!("Successfully created user {:?}.", user.name);
        },

        List => {
            if repo.users.is_empty() {
                println!("No users in the repository.");
                
                return Ok(());
            }

            println!("Users:");

            for user in repo.users.iter() {
                let mut line = format!(" * {}", user.name);

                if let Some(current_user) = repo.current_user() && current_user.name == user.name {
                    line = format!("{}", line.green());
                }

                println!("{line}");
            }
        },

        Info { username, show_private_key } => {
            let user = if let Some(name) = username {
                unwrap!(
                    repo.users.get_user(name.as_str()),
                    "no user with name {name:?} in this repository."
                )
            }
            else {
                unwrap!(
                    repo.current_user(),
                    "no valid user set on this repository."
                )
            };

            let name = if user.closed {
                format!("{} (closed)", user.name)
            }
            else {
                user.name.clone()
            };

            println!("Name: {name}");
            println!("Public key: {}", user.public_key);
            
            if show_private_key {
                println!("Private key: {}", match &user.private_key {
                    Some(key) => format!("{key}"),
                    None => "none".to_string()
                });
            }
        },

        Close { username } => {
            let user = unwrap!(
                repo.users.get_user_mut(&username),
                "no user with name {username:?} in this repository."
            );

            if user.closed {
                println!("User account is already closed.");
            }
            else {
                user.closed = true;

                println!("Closed user account {username:?}");
            }
        },

        Reopen { username } => {
            let user = unwrap!(
                repo.users.get_user_mut(&username),
                "no user with name {username:?} in this repository."
            );

            if user.closed {
                user.closed = false;

                println!("Reopened user account {username:?}");
            }
            else {
                println!("User account is already open.");
            }
        },

        Current { username: Some(name) } => {
            repo.set_current_user(&name)?;

            println!("Changed user to: {name:?}");
        },
        
        Current { username: None } => {
            if let Some(user) = repo.current_user() {
                println!("{}", user.name);
            }
            else {
                bail!("no valid user is set in this repository.");
            }
        },

        Rename { old, new } => {
            let user = unwrap!(
                repo.users.get_user_mut(&old),
                "no user in this repository with the name {old:?}"
            );

            user.name = new;

            println!("Renamed user: {old:?} -> {:?}", user.name);
        }
    }

    repo.save()?;

    Ok(())
}
