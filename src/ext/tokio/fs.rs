use std::{io::ErrorKind, path::Path};

use tokio::{
    fs::{File, OpenOptions},
    io,
};

pub(crate) trait FileExt: Sized {
    async fn open_write(path: impl AsRef<Path>) -> io::Result<Self>;

    async fn open_read_write(path: impl AsRef<Path>) -> io::Result<Self>;

    async fn open_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>>;

    async fn open_write_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>>;

    async fn open_read_write_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>>;
}

impl FileExt for File {
    async fn open_write(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new().write(true).open(path).await?;

        Ok(file)
    }

    async fn open_read_write(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path).await?;

        Ok(file)
    }

    async fn open_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let file = match Self::open(path).await {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        Ok(Some(file))
    }

    async fn open_write_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let file = match Self::open_write(path).await {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        Ok(Some(file))
    }

    async fn open_read_write_if_exists(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let file = match Self::open_read_write(path).await {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        Ok(Some(file))
    }
}
