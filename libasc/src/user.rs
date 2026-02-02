use eyre::{OptionExt, Result, bail};
use serde::{Deserialize, Serialize};

use crate::key::{PrivateKey, PublicKey};

/// Represents a user account in the repository.
#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub public_key: PublicKey,
    pub private_key: Option<PrivateKey>,
    pub closed: bool,
}

impl User {
    pub fn new(username: String) -> User {
        let private_key = PrivateKey::new();

        User {
            name: username,
            public_key: private_key.public_key(),
            private_key: Some(private_key),
            closed: false
        }
    }
}

pub enum SearchType<'data> {
    Username(&'data str),
    PublicKey(&'data PublicKey),
    PrivateKey(&'data PrivateKey)
}

impl<'data> SearchType<'data> {
    pub fn matches(&self, user: &User) -> bool {
        match self {
            SearchType::Username(name) => user.name == *name,
            SearchType::PublicKey(key) => user.public_key == **key,
            SearchType::PrivateKey(key) => user.private_key.as_ref() == Some(*key),
        }
    }
}

pub trait AsSearchType<'data> {
    fn as_search_type(&self) -> SearchType<'data>;
}

impl<'data> AsSearchType<'data> for &'data str {
    fn as_search_type(&self) -> SearchType<'data> {
        SearchType::Username(self)
    }
}

impl<'data> AsSearchType<'data> for &'data String {
    fn as_search_type(&self) -> SearchType<'data> {
        SearchType::Username(self.as_str())
    }
}

impl<'data> AsSearchType<'data> for &'data PublicKey {
    fn as_search_type(&self) -> SearchType<'data> {
        SearchType::PublicKey(self)
    }
}

impl<'data> AsSearchType<'data> for &'data PrivateKey {
    fn as_search_type(&self) -> SearchType<'data> {
        SearchType::PrivateKey(self)
    }
}

/// A collection of users for a repository.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Users {
    inner: Vec<User>,
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
    pub fn create_user(&mut self, username: String) -> Result<&mut User> {
        if self.get_user(username.as_str()).is_some() {
            bail!("username {username:?} already exists.");
        }

        if username.is_empty() {
            bail!("empty usernames are not allowed.");
        }

        let user = User::new(username);

        self.add_user(user)
    }

    /// Add a user to the repository.
    /// 
    /// This is usually done when the private keys are not stored locally,
    /// like when the user account is received from a remote.
    pub fn add_user(&mut self, user: User) -> Result<&mut User> {
        let search = self.iter().find(|u| {
            u.public_key == user.public_key || u.name == user.name
        });
        
        if search.is_some() {
            bail!("user with public key {:?} already exists.", user.public_key);
        }

        self.inner.push(user);

        self.inner
            .last_mut()
            .ok_or_eyre("")
    }

    pub fn get_user<'data>(&self, query: impl AsSearchType<'data>) -> Option<&User> {
        let search = query.as_search_type();

        self.inner.iter().find(|user| search.matches(user))
    }

    pub fn get_user_mut<'data>(&mut self, query: impl AsSearchType<'data>) -> Option<&mut User> {
        let search = query.as_search_type();

        self.inner.iter_mut().find(|user| search.matches(user))
    }

    /// Iterature through all [`User`]s in the repository.
    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.inner.iter()
    }

    /// Check if no users are in the repository.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Return a new [`Users`] where no account has a private key.
    pub fn without_private_keys(&self) -> Users {
        let mut users = Users::new();

        for user in self.iter() {
            let mut user = user.clone();

            user.private_key = None;

            users.add_user(user).unwrap();
        }
        
        users
    }
}
