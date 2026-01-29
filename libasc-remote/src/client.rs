#![allow(unused)]

use std::{collections::HashSet, sync::Arc};

use crate::{session::{LoginRequest, LoginResponse, MethodType}, utils::Repo};

use async_ssh2_tokio::Client as SshClient;
use eyre::{Result, bail};
use libasc::{hash::ObjectHash, sync::stream::{SshStream, Stream}, unwrap};
use tokio::{sync::{Mutex, mpsc::channel}, task::JoinHandle};

type SshResult = std::result::Result<u32, async_ssh2_tokio::Error>;

pub struct Client {
    conn: SshStream,
    repo: Repo,
}

static COMMAND: &str = "python3 /home/axo/dev/rust/vcs/test-server/main.py";

// macro_rules! error {
//     ($conn:expr, $($t:tt)*) => {{
//         let conn: &mut Connection = &mut $conn;

//         let message = format!($($t)*);

//         conn.send_messages(&[Message::Error(message)]).await?;

//         bail!($($t)*);
//     }};
// }

impl Client {
    pub async fn bind(ssh: SshClient, repo: Repo) -> Client {
        let (stdin_writer, stdin_reader) = channel(1024);
        let (stdout_writer, stdout_reader) = channel(1024);

        let handle = tokio::spawn(async move {
            ssh.execute_io(
                COMMAND,
                stdout_writer,
                None,
                Some(stdin_reader),
                false,
                None,
            )
            .await
        });

        let conn = SshStream::new(stdout_reader, stdin_writer);

        tokio::spawn(handle);

        Client { conn, repo }
    }

    pub async fn login(&mut self, method: MethodType) -> Result<()> {
        let mut repo = self.repo.lock().await;

        let user = unwrap!(repo.current_user(), "no user set on the repository.");

        let mut key = user.private_key.clone().unwrap();

        let signature = key.sign(repo.project_code.as_bytes());

        let login = LoginRequest {
            project_code: repo.project_code,
            user: signature,
            method,
        };

        self.conn.send(&login).await?;

        let response: LoginResponse = self.conn.receive().await?;

        match response {
            Ok(reply) => {
                // FIXME
                // repo.users = reply.users.into();
            }

            Err(message) => bail!("Error from server: {message}"),
        }

        Ok(())
    }

    pub async fn make_pull(&mut self) -> Result<()> {
        // TODO: make this use RIBLTs instead of a naive hashset approach

        Ok(())
    }
}
