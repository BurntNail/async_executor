use std::path::Path;
use crate::id::Id;
use crate::adapters::io_worker::{IoReqOptions, IoOutcome};
use crate::adapters::SimpleThreadFuture;

pub struct File {
    id: Id,
}

impl File {
    pub async fn open (p: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let id = SimpleThreadFuture::new(
            IoReqOptions::OpenFile(p.as_ref().to_path_buf()),
            |outcome| {
                if let IoOutcome::FileOpenedOrCreated(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await?;
        Ok(Self {
            id
        })
    }

    pub async fn create (path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let id = SimpleThreadFuture::new(
            IoReqOptions::CreateFile(path.as_ref().to_path_buf()),
            |outcome| {
                if let IoOutcome::FileOpenedOrCreated(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await?;
        Ok(Self {
            id
        })
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub async fn read (&mut self, max_bytes: usize) -> Result<Vec<u8>, std::io::Error> {
        SimpleThreadFuture::new(
            IoReqOptions::ReadFile(self.id, max_bytes),
            |outcome| {
                if let IoOutcome::FileBytesRead(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub async fn read_to_end (&mut self) -> Result<Vec<u8>, std::io::Error> {
        SimpleThreadFuture::new(
            IoReqOptions::FullyReadFile(self.id),
            |outcome| {
                if let IoOutcome::FileBytesRead(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub async fn write (&mut self, buf: Vec<u8>) -> Result<usize, std::io::Error> {
        SimpleThreadFuture::new(
            IoReqOptions::WriteFile(self.id, buf),
            |outcome| {
                if let IoOutcome::FileBytesWritten(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }

    pub async fn write_all (&mut self, mut buf: &[u8]) -> Result<(), std::io::Error> {
        while !buf.is_empty() {
            match self.write(buf.to_vec()).await {
                Ok(0) => {
                    return Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    pub async fn metadata (&self) -> Result<std::fs::Metadata, std::io::Error> {
        SimpleThreadFuture::new(
            IoReqOptions::FileMetadata(self.id),
            |outcome| {
                if let IoOutcome::FileMetadata(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }

    pub async fn into_std(self) -> std::fs::File {
        SimpleThreadFuture::new(
            IoReqOptions::FileToStd(self.id),
            |outcome| {
                if let IoOutcome::FileToStd(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }
    
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub async fn set_len (&mut self, size: u64) -> Result<(), std::io::Error> {
        SimpleThreadFuture::new(
            IoReqOptions::SetFileLen(self.id, size),
            |outcome| {
                if let IoOutcome::FileLenSet(res) = outcome {
                    Some(res)
                } else {
                    None
                }
            }
        ).await
    }
}