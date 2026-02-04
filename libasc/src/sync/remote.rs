use std::{fmt::Display, path::PathBuf};

use eyre::{Result, bail};
use git_url_parse::GitUrl;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Remote {
    username: String,
    password: Option<String>,
    host: String,
    port: u16,
    repo_path: PathBuf
}

impl Remote {
    pub fn new(
        username: String,
        password: Option<String>,
        host: String,
        port: Option<u16>,
        repo_path: PathBuf
    ) -> Self
    {
        Self {
            username,
            password,
            host,
            port: port.unwrap_or(22),
            repo_path
        }
    }

    pub fn ssh_url(&self) -> String {
        let mut login = self.username.clone();
        
        if let Some(password) = &self.password {
            login = format!("{login}:{password}");
        };

        format!("ssh://{login}@{}:{}", self.host, self.port)
    }

    pub fn path(&self) -> String {
        format!("{}", self.repo_path.display())
    }

    pub fn from_url(url: &str) -> Result<Self> {
        let parsed = GitUrl::parse(url)?;

        let Some(host) = parsed.host() else {
            bail!("missing host in URL: {url:?}");
        };

        let username = parsed.user()
            .map(String::from)
            .unwrap_or("asc".to_string());

        let password = parsed.password()
            .map(String::from);

        Ok(Remote::new(
            username,
            password,
            host.to_string(),
            parsed.port(),
            PathBuf::from(parsed.path())
        ))
    }
}

impl Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pass) = &self.password {
            let mut path = self.repo_path.display().to_string();

            if path.starts_with('/') {
                path = path.split_off(1);
            }

            write!(f, "ssh://{}:{}@{}/{}", self.username, pass, self.host, path)
        }
        else {
            write!(f, "{}@{}:{}", self.username, self.host, self.port)
        }
    }
}
