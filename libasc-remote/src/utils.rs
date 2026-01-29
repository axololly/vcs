use std::sync::Arc;

use libasc::repository::Repository;
use tokio::sync::Mutex;

// use crate::session::Message;

// pub type Messages = Vec<Message>;

pub type ArcMut<T> = Arc<Mutex<T>>;

pub type Repo = ArcMut<Repository>;

#[macro_export]
macro_rules! read_from {
    ($stream:expr) => {{
        use tokio::{io::AsyncReadExt, net::TcpStream, sync::MutexGuard};

        let mut stream: MutexGuard<'_, TcpStream> = $stream;
        
        stream.readable().await?;
    
        let size = stream.read_u64().await?;

        let mut buf = vec![0u8; size as usize];

        stream.read_exact(&mut buf).await?;

        buf
    }};
}

#[macro_export]
macro_rules! write_to {
    ($stream:expr, $data:expr) => {{
        use tokio::{io::AsyncWriteExt, net::TcpStream, sync::MutexGuard};

        let mut stream: MutexGuard<'_, TcpStream> = $stream;
        
        let data: &[u8] = $data;

        stream.writable().await?;

        let size = data.len() as u64;

        stream.write_u64(size).await?;

        stream.write_all(data).await?;

        stream.flush().await?;
    }};
}
