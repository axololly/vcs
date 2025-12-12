use std::{collections::HashMap, sync::LazyLock};

use argon2::{
    password_hash::{
        rand_core::OsRng, Error::Password as InvalidPassword, PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use bitflags::bitflags;
use derive_more::Display;
use eyre::Result;
use serde::{Deserialize, Serialize};

fn hash_password<'a>(password: &'a str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;

    Ok(password_hash.to_string())
}

fn check_password(attempt: &str, hash: &str) -> Result<bool> {
    let password_hash = PasswordHash::new(&hash)?;

    let ctx = Argon2::default();

    let result = ctx.verify_password(attempt.as_bytes(), &password_hash);

    match result {
        Ok(_) => Ok(true),
        Err(InvalidPassword) => Ok(false),
        Err(e) => Err(From::from(e))
    }
}

/// Generate a random alphanumeric string to be used as a
/// template password for default accounts.
/// 
/// These are meant to be overwritten by the user at a later date.
pub fn get_random_password(length: usize) -> String {
    let valid = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    let bytes = std::iter::repeat_with(|| {
            valid.as_bytes()[rand::random::<u8>() as usize]
        })
        .take(length)
        .collect();

    String::from_utf8(bytes).unwrap()
}

bitflags! {
    /// Represents the permissions a user in the repository has.
    #[derive(Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
    pub struct Permissions: u8 {
        const CAN_PUSH = 1 << 0;
        const CAN_PULL = 1 << 1;
    }
}

static LETTER_TO_PERMISSION: LazyLock<HashMap<char, Permissions>> = LazyLock::new(|| {
    HashMap::from([
        ('c', Permissions::CAN_PULL),
        ('p', Permissions::CAN_PUSH)
    ])
});

impl Permissions {
    pub fn can_push(&self) -> bool {
        self.contains(Permissions::CAN_PUSH)
    }

    pub fn can_pull(&self) -> bool {
        self.contains(Permissions::CAN_PULL)
    }

    pub fn to_string_pretty(&self) -> String {
        let inverted: HashMap<Permissions, char> = LETTER_TO_PERMISSION
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect();

        self.iter()
            .map(|p| inverted[&p])
            .collect()
    }
}

#[derive(Debug, Display)]
pub enum Error {
    #[display("invalid permission string: {_0:?} (invalid permission char: {_1:?})")]
    InvalidPermissionString(String, char),

    #[display("user with username {_0:?} already exists in the repository.")]
    UsernameAlreadyExists(String)
}

impl std::error::Error for Error {}

impl TryFrom<&str> for Permissions {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let mut perms = Permissions::empty();

        for ch in value.chars() {
            let perm = LETTER_TO_PERMISSION
                .get(&ch)
                .cloned()
                .ok_or(Error::InvalidPermissionString(value.to_string(), ch))?;

            perms |= perm;
        }
        
        Ok(perms)
    }
}

impl TryFrom<String> for Permissions {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Permissions::try_from(value.as_str())
    }
}

/// Represents a user account in the repository.
#[derive(Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub password: String,
    pub permissions: Permissions
}

impl User {
    /// Create a new [`User`] with empty [`Permissions`].
    /// 
    /// To create a [`User`] with specified [`Permissions`],
    /// use [`User::with_permissions`].
    pub fn new(username: &str, password: &str) -> Result<User> {
        User::with_permissions(
            username,
            password,
            Permissions::empty()
        )
    }

    /// Create a new [`User`] with a given set of [`Permissions`].
    pub fn with_permissions(
        username: &str,
        password: &str,
        permissions: Permissions
    ) -> Result<User>
    {
        let name = username.to_string();

        let password = hash_password(password)?;

        Ok(User {
            name,
            password,
            permissions
        })
    }

    /// Check if a given password is the correct password for this account.
    pub fn check_password(&self, password: &str) -> Result<bool> {
        check_password(password, &self.password)
    }

    /// Overwrite the existing password for this [`User`].
    pub fn change_password(&mut self, password: &str) -> Result<()> {
        self.password = hash_password(password)?;

        Ok(())
    }
}

/// A collection of users for a repository.
#[derive(Deserialize, Serialize)]
pub struct Users {
    inner: Vec<User>
}

impl Users {
    /// Create a new [`Users`] with no user accounts.
    pub fn new() -> Users {
        Users { inner: vec![] }
    }

    /// Create a new [`User`] for the repository.
    /// 
    /// These get given empty permissions. To change that user's permissions,
    /// use [`Users::get_user_mut`] and update [`User::permissions`], or insert
    /// the permissions with [`Users::create_user_with_permissions`]
    pub fn create_user(&mut self, username: &str, password: &str) -> Result<()> {
        let user = User::new(username, password)?;
        
        if self.get_user(username).is_some() {
            return Err(From::from(Error::UsernameAlreadyExists(username.to_string())));
        }

        self.inner.push(user);

        Ok(())
    }

    /// Create a new [`User`] for the repository with a given set of permissions.
    pub fn create_user_with_permissions(
        &mut self,
        username: &str,
        password: &str,
        permissions: Permissions
    ) -> Result<()>
    {
        self.create_user(username, password)?;

        let user = self.get_user_mut(username).unwrap();

        user.permissions = permissions;

        Ok(())
    }

    /// Get a [`User`] from the repository.
    pub fn get_user(&self, username: &str) -> Option<&User> {
        self.inner
            .iter()
            .find(|u| u.name == username)
    }

    /// Get a mutable [`User`] from the repository.
    pub fn get_user_mut(&mut self, username: &str) -> Option<&mut User> {
        self.inner
            .iter_mut()
            .find(|u| u.name == username)
    }

    /// Remove and return a [`User`] from the repository.
    pub fn remove_user(&mut self, username: &str) -> Option<User> {
        let index = self.inner
            .iter()
            .position(|u| u.name == username)?;

        Some(self.inner.swap_remove(index))
    }

    /// Iterature through all [`User`]s in the repository.
    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.inner.iter()
    }

    /// Check if no users are in the repository.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}