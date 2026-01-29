use std::io;

use async_trait::async_trait;
use eyre::Result;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{
        AsyncReadExt as Read, AsyncWriteExt as Write, ReadHalf, SimplexStream, WriteHalf, simplex,
    },
    sync::mpsc::{Receiver, Sender},
};

#[async_trait]
pub trait Stream: Send {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>>;

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()>;

    async fn read(&mut self) -> io::Result<Vec<u8>> {
        let header = {
            let bytes = self.raw_read(8).await?;

            let bytes = bytes.try_into().unwrap();

            usize::from_be_bytes(bytes)
        };

        self.raw_read(header).await
    }

    async fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let header = bytes.len().to_be_bytes();

        self.raw_write(&header).await?;

        self.raw_write(bytes).await
    }

    async fn close(&mut self) -> io::Result<()>;

    async fn send<T: Serialize + Sync>(&mut self, object: &T) -> Result<()> {
        let bytes = rmp_serde::to_vec(object)?;

        // println!("sending object {} (rmp: {} bytes)", std::any::type_name::<T>(), bytes.len());

        self.write(&bytes).await?;

        Ok(())
    }

    async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.read().await?;

        // println!("trying to read object {} (rmp: {} bytes)", std::any::type_name::<T>(), bytes.len());

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

#[async_trait]
impl Stream for SshStream {
    async fn raw_read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut bytes = self.read_extra.split_off(n);

        if bytes.len() == n {
            return Ok(bytes);
        }

        let mut recv_bytes = self.reader.recv().await.ok_or(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "other side closed the connection",
        ))?;

        bytes.extend(recv_bytes);

        let extra = bytes.split_off(n);

        self.read_extra.extend(extra);

        Ok(bytes)
    }

    async fn raw_write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer
            .send(bytes.to_vec())
            .await
            .map_err(|e| io::Error::new(
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
