use std::io;

use async_trait::async_trait;
use eyre::Result;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{io::{AsyncReadExt as Read, AsyncWriteExt as Write, ReadHalf, SimplexStream, Stdin, Stdout, WriteHalf, simplex, stdin, stdout}, sync::mpsc::{Receiver, Sender}};

#[async_trait]
pub trait Stream: Send {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>>;

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()>;

    async fn read(&mut self) -> io::Result<Vec<u8>> {
        let header = {
            let bytes = self.raw_read(8).await?;

            assert!(bytes.len() == 8);

            let bytes = bytes.try_into().unwrap();

            usize::from_le_bytes(bytes)
        };

        self.raw_read(header).await
    }

    async fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let header = bytes.len().to_le_bytes();

        self.raw_write(&header).await?;

        self.raw_write(bytes).await
    }

    async fn close(&mut self) -> io::Result<()>;

    async fn send<T: Serialize + Sync>(&mut self, object: &T) -> Result<()> {
        let bytes = rmp_serde::to_vec(object)?;

        // println!(
        //     "sending object {} (rmp: {} bytes)",
        //     std::any::type_name::<T>(),
        //     bytes.len()
        // );

        self.write(&bytes).await?;

        Ok(())
    }

    async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.read().await?;

        // println!(
        //     "trying to read object {} (rmp: {} bytes)",
        //     std::any::type_name::<T>(),
        //     bytes.len()
        // );

        let object = rmp_serde::from_slice(&bytes)?;

        Ok(object)
    }
}

pub struct LocalStream {
    reader: ReadHalf<SimplexStream>,
    writer: WriteHalf<SimplexStream>,
}

impl LocalStream {
    pub fn new(reader: ReadHalf<SimplexStream>, writer: WriteHalf<SimplexStream>) -> Self {
        Self { reader, writer }
    }
}

#[async_trait]
impl Stream for LocalStream {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];

        self.reader
            .read_exact(&mut buf)
            .await
            .map(|_| buf)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e))
    }

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer
            .write_all(bytes)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e))
    }

    async fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn local_duplex() -> (LocalStream, LocalStream) {
    let (client_read, client_write) = simplex(1024);
    let (server_read, server_write) = simplex(1024);

    let client = LocalStream::new(client_read, server_write);
    let server = LocalStream::new(server_read, client_write);

    (client, server)
}

pub struct SshStream {
    reader: Receiver<Vec<u8>>,
    writer: Sender<Vec<u8>>,
    read_extra: Vec<u8>,
}

impl SshStream {
    pub fn new(reader: Receiver<Vec<u8>>, writer: Sender<Vec<u8>>) -> Self {
        Self {
            reader,
            writer,
            read_extra: vec![]
        }
    }
}

fn take_n_bytes(data: &mut Vec<u8>, n: usize) -> Vec<u8> {
    // data holds what we will return, while
    // after_n holds what we want to keep
    let mut after_n = data.split_off(n);

    // now data holds what we want to keep and
    // after_n holds what we want to return
    std::mem::swap(data, &mut after_n);

    after_n
}

#[async_trait]
impl Stream for SshStream {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut bytes = if n < self.read_extra.len() {
            take_n_bytes(&mut self.read_extra, n)
        }
        else {
            std::mem::take(&mut self.read_extra)
        };

        while bytes.len() < n {
            let recv_bytes = self.reader
                .recv()
                .await
                .ok_or(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "other side closed the connection",
                ))?;

            bytes.extend(recv_bytes);
        }

        let taken = take_n_bytes(&mut bytes, n);

        self.read_extra = bytes;

        Ok(taken)
    }

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer
            .send(bytes.to_vec())
            .await
            .map_err(|_| io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "other side closed the connection"
            ))
    }

    async fn close(&mut self) -> io::Result<()> {
        self.reader.close();
        self.writer.closed().await;

        Ok(())
    }
}

pub struct StdinStdout {
    reader: Stdin,
    writer: Stdout
}

impl Default for StdinStdout {
    fn default() -> Self {
        Self {
            reader: stdin(),
            writer: stdout()
        }
    }
}

impl StdinStdout {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Stream for StdinStdout {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];

        self.reader.read_exact(&mut buf).await?;

        Ok(buf)
    }

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes).await?;

        self.writer.flush().await
    }

    async fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}
