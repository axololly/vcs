use std::{collections::HashMap, fmt::Display, sync::LazyLock};

use bitflags::bitflags;
use derive_more::Display;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::key::{PrivateKey, PublicKey};

bitflags! {
    /// Represents the permissions a user in the repository has.
    #[derive(Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
    pub struct Permissions: u8 {
        const CAN_PUSH = 1 << 0;
        const CAN_PULL = 1 << 1;
    }
}

static LETTER_TO_PERMISSION: LazyLock<HashMap<char, Permissions>> =
    LazyLock::new(|| HashMap::from([('c', Permissions::CAN_PULL), ('p', Permissions::CAN_PUSH)]));

impl Permissions {
    pub fn can_push(&self) -> bool {
        self.contains(Permissions::CAN_PUSH)
    }

    pub fn can_pull(&self) -> bool {
        self.contains(Permissions::CAN_PULL)
    }
}

impl Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inverted: HashMap<Permissions, char> =
            LETTER_TO_PERMISSION.iter().map(|(&k, &v)| (v, k)).collect();

        for p in *self {
            write!(f, "{}", inverted[&p])?;
        }

        Ok(())
    }
}

#[derive(Debug, Display)]
pub enum Error {
    #[display("invalid permission string: {_0:?} (invalid permission char: {_1:?})")]
    InvalidPermissionString(String, char),

    #[display("user with username {_0:?} already exists in the repository.")]
    UsernameAlreadyExists(String),

    #[display("cannot have an account with an empty username")]
    EmptyUsernameDisallowed,
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
#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub permissions: Permissions,
    pub public_key: PublicKey,
    pub private_key: Option<PrivateKey>,
    pub closed: bool,
}

impl User {
    /// Create a new [`User`] with empty [`Permissions`].
    ///
    /// To create a [`User`] with specified [`Permissions`],
    /// use [`User::with_permissions`].
    pub fn new(username: String) -> User {
        User::with_permissions(username, Permissions::empty())
    }

    /// Create a new [`User`] with a given set of [`Permissions`].
    pub fn with_permissions(username: String, permissions: Permissions) -> User {
        let private_key = PrivateKey::new();

        User {
            name: username,
            permissions,
            public_key: private_key.public_key(),
            private_key: Some(private_key),
            closed: false,
        }
    }
}

/// A collection of users for a repository.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Users {
    inner: HashMap<PublicKey, User>,
}

impl Users {
    /// Create a new [`Users`] with no user accounts.
    pub fn new() -> Users {
        Users::default()
    }

    /// Create a new [`User`] for the repository.
    ///
    /// These get given empty permissions. To change that user's permissions,
    /// use [`Users::get_user_mut`] and update [`User::permissions`], or insert
    /// the permissions with [`Users::create_user_with_permissions`]
    pub fn create_user(&mut self, username: String) -> std::result::Result<&mut User, Error> {
        if self.get_user(&username).is_some() {
            return Err(Error::UsernameAlreadyExists(username.to_string()));
        }

        if username.is_empty() {
            return Err(Error::EmptyUsernameDisallowed);
        }

        let user = User::new(username);

        let key = user.public_key;

        self.inner.insert(key, user);

        Ok(self.inner.get_mut(&key).unwrap())
    }

    /// Add a user to the repository.
    /// 
    /// This is usually done when the private keys are not stored locally,
    /// like when the user account is received from a remote.
    pub fn add_user(
        &mut self,
        name: String,
        permissions: Permissions,
        public_key: PublicKey,
        private_key: Option<PrivateKey>
    ) -> Result<&mut User>
    {
        let user = User {
            name,
            permissions,
            public_key,
            private_key,
            closed: true
        };

        self.inner.insert(public_key, user);

        Ok(self.inner.get_mut(&public_key).unwrap())
    }

    /// Create a new [`User`] for the repository with a given set of permissions.
    pub fn create_user_with_permissions(
        &mut self,
        username: String,
        permissions: Permissions,
    ) -> Result<&mut User>
    {
        let user = self.create_user(username)?;

        user.permissions = permissions;

        Ok(user)
    }

    /// Check if a [`User`] by a given `username` exists in the repository.
    pub fn has_user(&self, username: &str) -> bool {
        self.get_user(username).is_some()
    }

    /// Get a [`User`] from the repository.
    pub fn get_user(&self, username: &str) -> Option<&User> {
        self.inner.values().find(|u| u.name == username)
    }

    /// Get a [`User`] by searching for a matching public key instead of a username.
    pub fn get_user_by_pub_key(&self, public_key: PublicKey) -> Option<&User> {
        self.inner.get(&public_key)
    }

    /// Get a mutable [`User`] from the repository.
    pub fn get_user_mut(&mut self, username: &str) -> Option<&mut User> {
        self.inner.values_mut().find(|u| u.name == username)
    }

    /// Get a [`User`] by searching for a matching public key instead of a username.
    pub fn get_user_mut_by_pub_key(&mut self, public_key: PublicKey) -> Option<&mut User> {
        self.inner.get_mut(&public_key)
    }

    /// Remove and return a [`User`] from the repository.
    pub fn remove_user(&mut self, username: &str) -> Option<User> {
        let user = self.get_user(username)?;

        let key = user.public_key;

        self.inner.remove(&key)
    }

    /// Iterature through all [`User`]s in the repository.
    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.inner.values()
    }

    /// Check if no users are in the repository.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl From<Vec<User>> for Users {
    fn from(value: Vec<User>) -> Self {
        let inner = value
            .into_iter()
            .map(|u| (u.public_key, u))
            .collect();

        Users { inner }
    }
}
