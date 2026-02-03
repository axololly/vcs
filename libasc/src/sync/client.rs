use std::{path::Path, sync::Arc};

use async_ssh2_tokio::{AuthMethod, Client as SshClient, ServerCheckMethod};
use eyre::Result;
use tokio::sync::{Mutex, mpsc::channel};
use url_parse::core::Parser;

use crate::{key::PrivateKey, repository::Repository, sync::{clone::handle_clone_as_client, entry::Method, pull::{PullResult, handle_pull_as_client}, push::{PushResult, handle_push_as_client}, stream::{SshStream, Stream}}};

type Repo = Arc<Mutex<Repository>>;

pub struct Client {
    conn: SshStream
}

impl Client {
    pub async fn connect(endpoint: &str) -> Result<Client> {
        let url = Parser::new(None).parse(endpoint)?;
        
        let base_url = {
            let mut parts: Vec<String> = vec![];

            if let Some(subdomain) = &url.subdomain {
                parts.push(subdomain.clone());
            }

            if let Some(host_str) = url.host_str() {
                parts.push(host_str);
            }

            parts.join(".")
        };

        let username = url
            .username()
            .unwrap_or("axo".to_string()); // TODO: change to asc

        let auth = if let Some(password) = url.password() {
            AuthMethod::Password(password)
        }
        else {
            AuthMethod::Agent
        };

        let port = url
            .port_or_known_default()
            .unwrap_or(22);

        let ssh = SshClient::connect(
            (base_url.as_str(), port as u16),
            &username,
            auth,
            ServerCheckMethod::DefaultKnownHostsFile
        ).await?;

        let (stdin_writer, stdin_reader) = channel(1024);
        let (stdout_writer, stdout_reader) = channel(1024);

        tokio::spawn(async move {
            let _ = ssh.execute_io(
                "cd ~/dev/rust/vcs && ./remote-handler",
                stdout_writer,
                None,
                Some(stdin_reader),
                false,
                None,
            )
            .await;
        });

        let conn = SshStream::new(stdout_reader, stdin_writer);

        Ok(Client { conn })
    }

    pub async fn make_pull(&mut self, repo: Repo) -> Result<Vec<PullResult>> {
        self.conn.send(&Method::Pull).await?;

        handle_pull_as_client(&mut self.conn, repo).await
    }

    pub async fn make_push(&mut self, repo: Repo) -> Result<Vec<PushResult>> {
        self.conn.send(&Method::Push).await?;

        handle_push_as_client(&mut self.conn, repo).await
    }

    pub async fn clone_repo(&mut self, repo_path: &Path, user_key: PrivateKey) -> Result<()> {
        self.conn.send(&Method::Clone).await?;

        handle_clone_as_client(&mut self.conn, repo_path, user_key).await
    }
}
