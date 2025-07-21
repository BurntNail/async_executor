use std::path::Path;
use crate::adapters::file::File;
use crate::adapters::io_worker::{DirChange, IoOutcome, IoReqOptions};
use crate::adapters::SimpleThreadFuture;

pub async fn read_to_string (p: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let mut file = File::open(p).await?;
    let contents = file.read_to_end().await?;
    String::from_utf8(contents).map_err(|utf8_error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            utf8_error
        )
    })
}

pub async fn metadata (p: impl AsRef<Path>) -> Result<std::fs::Metadata, std::io::Error> {
    let file = File::open(p.as_ref()).await?;
    file.metadata().await
}

pub async fn create_dir (p: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::ChangeDir(p.as_ref().to_path_buf(), DirChange::CreateDir),
        |outcome| {
            if let IoOutcome::DirChanged(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}
pub async fn create_dir_all (p: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::ChangeDir(p.as_ref().to_path_buf(), DirChange::CreateDirAll),
        |outcome| {
            if let IoOutcome::DirChanged(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}
pub async fn remove_dir (p: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::ChangeDir(p.as_ref().to_path_buf(), DirChange::RemoveDir),
        |outcome| {
            if let IoOutcome::DirChanged(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}
pub async fn remove_dir_all (p: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::ChangeDir(p.as_ref().to_path_buf(), DirChange::RemoveDirAll),
        |outcome| {
            if let IoOutcome::DirChanged(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}


pub async fn copy (from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64, std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::Copy(from.as_ref().to_path_buf(), to.as_ref().to_path_buf()),
        |outcome| {
            if let IoOutcome::Copied(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}

pub async fn remove_file(p: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::RemoveFile(p.as_ref().to_path_buf()),
        |outcome| {
            if let IoOutcome::Removed(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}

pub async fn rename (from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), std::io::Error> {
    SimpleThreadFuture::new(
        IoReqOptions::Rename(from.as_ref().to_path_buf(), to.as_ref().to_path_buf()),
        |outcome| {
            if let IoOutcome::Renamed(res) = outcome {
                Some(res)
            } else {
                None
            }
        }
    ).await
}