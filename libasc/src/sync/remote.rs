use std::{fmt::Display, path::PathBuf};

use eyre::{Result, bail};
use git_url_parse::GitUrl;
use serde::{Deserialize, Serialize};

use crate::unwrap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SshRemote {
    username: String,
    password: Option<String>,
    host: String,
    port: u16,
    repo_path: PathBuf
}

impl SshRemote {
    pub fn url(&self) -> String {
        let password = if let Some(pass) = &self.password {
            format!(":{pass}")
        }
        else {
            String::new()
        };

        let port = if self.port != 22 {
            format!(":{}", self.port)
        }
        else {
            String::new()
        };
        
        format!(
            "ssh://{}{password}@{}{port}",
            self.username,
            self.host,
        )
    }

    pub fn path(&self) -> &PathBuf {
        &self.repo_path
    }
}

impl Display for SshRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path = self.repo_path.display().to_string();

        if path.starts_with('/') {
            path = path.split_off(1);
        }

        write!(f, "{}/{}", self.url(), path)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileRemote {
    path: PathBuf
}

impl FileRemote {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Display for FileRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Remote {
    Ssh(SshRemote),
    File(FileRemote)
}

impl Remote {
    fn try_parse_ssh_url(url: &GitUrl) -> Result<Self> {
        let username = url
            .user()
            .unwrap_or("asc")
            .to_string();

        let password = url
            .password()
            .map(String::from);

        let host = unwrap!(
            url.host().map(String::from),
            "host missing from SSH URL"
        );

        let ssh_remote = SshRemote {
            username,
            password,
            host,
            port: url.port().unwrap_or(22),
            repo_path: PathBuf::from(url.path())
        };

        Ok(Remote::Ssh(ssh_remote))
    }

    fn try_parse_file_url(url: &GitUrl) -> Result<Self> {
        let path = PathBuf::from(url.path());

        let file_remote = FileRemote { path };

        Ok(Remote::File(file_remote))
    }

    pub fn from_url(url: &str) -> Result<Self> {
        let parsed = GitUrl::parse(url)?;

        match parsed.scheme() {
            Some("ssh") => Remote::try_parse_ssh_url(&parsed),
            Some("file") => Remote::try_parse_file_url(&parsed),

            Some("http" | "https") => bail!("HTTP URLs are unsupported."),

            Some(unknown) => bail!("unknown scheme identified: {unknown:?}"),
            None => bail!("no scheme identified.")
        }
    }
}

impl Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Remote::Ssh(ssh) => {
                let mut path = ssh.repo_path.display().to_string();

                if path.starts_with('/') {
                    path = path.split_off(1);
                }

                write!(f, "ssh://{}@{}/{}", ssh.username, ssh.host, path)
            },
            
            Remote::File(FileRemote { path }) => {
                write!(f, "file://{}", path.display())
            }
        }
    }
}
