use std::{path::Path, process::Stdio, sync::Arc};

use eyre::Result;
use tokio::{process::Command, sync::Mutex};

use crate::{key::PrivateKey, repository::Repository, sync::{clone::handle_clone_as_client, pull::{PullResult, handle_pull_as_client}, push::{PushResult, handle_push_as_client}, remote::Remote, server::Method, stream::{ChildProcessStream, Stream}}};

type Repo = Arc<Mutex<Repository>>;

pub struct Client {
    conn: ChildProcessStream,
    remote: Remote
}

impl Client {
    pub async fn connect(remote: Remote, ssh_bin_path: Option<&str>) -> Result<Client> {
        let address = remote.ssh_url();

        let mut ssh = {
            let mut proc = Command::new(
                ssh_bin_path.unwrap_or("ssh")
            );

            proc.args([
                &address,
                "asc-server",
                &remote.path()
            ]);

            proc.stdin(Stdio::piped());

            proc.stdout(Stdio::piped());

            proc.spawn()?
        };

        let stdin = ssh.stdin.take().unwrap();
        let stdout = ssh.stdout.take().unwrap();

        let conn = ChildProcessStream::new(stdout, stdin);

        tokio::spawn(async move {
            ssh.wait().await
        });

        Ok(Client { conn, remote })
    }

    pub async fn make_pull(&mut self, repo: Repo) -> Result<Vec<PullResult>> {
        self.conn.send(&Method::Pull).await?;

        handle_pull_as_client(&mut self.conn, repo).await
    }

    pub async fn make_push(&mut self, repo: Repo) -> Result<Vec<PushResult>> {
        self.conn.send(&Method::Push).await?;

        handle_push_as_client(&mut self.conn, repo).await
    }

    pub async fn clone_repo(&mut self, local_repo_path: &Path, user_key: PrivateKey) -> Result<()> {
        self.conn.send(&Method::Clone).await?;

        handle_clone_as_client(
            &mut self.conn,
            self.remote.clone(),
            local_repo_path,
            user_key
        ).await
    }
}
