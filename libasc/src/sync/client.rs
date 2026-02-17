use std::{io, path::Path, process::Stdio, sync::Arc};

use async_trait::async_trait;
use eyre::Result;
use tokio::{process::Command, sync::Mutex};

use crate::{key::PrivateKey, repository::Repository, sync::{clone::handle_clone_as_client, pull::{handle_pull_as_client, PullResult}, push::{handle_push_as_client, PushResult}, remote::{FileRemote, Remote, SshRemote}, server::{handle_server, Method}, stream::{local_duplex, ChildProcessStream, LocalStream, Stream}}};

type Repo = Arc<Mutex<Repository>>;

pub enum Connection {
    Ssh(ChildProcessStream),
    File(LocalStream)
}

#[async_trait]
impl Stream for Connection {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        match self {
            Self::Ssh(stream) => stream.raw_read(n).await,
            Self::File(stream) => stream.raw_read(n).await
        }
    }

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::Ssh(stream) => stream.raw_write(bytes).await,
            Self::File(stream) => stream.raw_write(bytes).await
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        match self {
            Self::Ssh(stream) => stream.close().await,
            Self::File(stream) => stream.close().await
        }
    }
}

pub struct Client {
    conn: Connection,
    remote: Remote
}

impl Client {
    async fn connect_ssh(remote: SshRemote) -> Result<Client> {
        let address = remote.to_string();

        let mut ssh = {
            let mut proc = Command::new("ssh");

            proc.args([
                address,
                "asc-server".to_string(),
                format!("{}", remote.path().display())
            ]);

            proc.stdin(Stdio::piped());

            proc.stdout(Stdio::piped());

            proc.spawn()?
        };

        let stdin = ssh.stdin.take().unwrap();
        let stdout = ssh.stdout.take().unwrap();

        let stream = ChildProcessStream::new(stdout, stdin);

        tokio::spawn(async move {
            ssh.wait().await
        });

        let conn = Connection::Ssh(stream);

        let remote = Remote::Ssh(remote);

        Ok(Client { conn, remote })
    }

    async fn connect_file(remote: FileRemote) -> Result<Client> {
        let remote_repo = Arc::new(Mutex::new(
            Repository::load_from(remote.path())?
        ));

        let (stream, mut server) = local_duplex();

        tokio::spawn(async move {
            handle_server(
                &mut server,
                remote_repo.clone()
            ).await
        });

        let conn = Connection::File(stream);

        let remote = Remote::File(remote);

        Ok(Client { conn, remote })
    }

    pub async fn connect(remote: Remote) -> Result<Client> {
        match remote {
            Remote::File(rem) => Client::connect_file(rem).await,
            Remote::Ssh(rem) => Client::connect_ssh(rem).await
        }
    }

    pub async fn make_pull(&mut self, repo: Repo) -> Result<Vec<PullResult>> {
        self.conn.send(&Method::Pull).await?;

        handle_pull_as_client(&mut self.conn, repo).await
    }

    pub async fn make_push(&mut self, repo: Repo) -> Result<Vec<PushResult>> {
        self.conn.send(&Method::Push).await?;

        handle_push_as_client(&mut self.conn, repo).await
    }

    pub async fn clone_repo(
        &mut self,
        local_repo_path: &Path,
        user_key: PrivateKey
    ) -> Result<Repository>
    {
        self.conn.send(&Method::Clone).await?;

        handle_clone_as_client(
            &mut self.conn,
            self.remote.clone(),
            local_repo_path,
            user_key
        ).await?;

        Repository::load_from(local_repo_path)
    }
}
