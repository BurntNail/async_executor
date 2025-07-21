use std::collections::HashMap;
use std::fs::File as StdFile;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::task::Waker;
use crate::id::{Id, IdGenerator};

pub struct IoReq {
    waker: Waker,
    options: IoReqOptions
}

pub enum IoReqOptions {
    OpenFile(PathBuf),
    CreateFile(PathBuf),
    ReadFile(Id, usize),
    FullyReadFile(Id),
    WriteFile(Id, Vec<u8>),
    FileMetadata(Id),
    FileToStd(Id),
    SetFileLen(Id, u64),
    ChangeDir(PathBuf, DirChange),
    Copy(PathBuf, PathBuf),
    RemoveFile(PathBuf),
    Rename(PathBuf, PathBuf),
}

pub enum DirChange {
    CreateDir,
    CreateDirAll,
    RemoveDir,
    RemoveDirAll,
}

pub enum IoOutcome {
    FileOpenedOrCreated(Result<Id, std::io::Error>),
    FileBytesRead(Result<Vec<u8>, std::io::Error>),
    FileBytesWritten(Result<usize, std::io::Error>),
    FileMetadata(Result<std::fs::Metadata, std::io::Error>),
    FileToStd(StdFile),
    FileLenSet(Result<(), std::io::Error>),
    DirChanged(Result<(), std::io::Error>),
    Copied(Result<u64, std::io::Error>),
    Removed(Result<(), std::io::Error>),
    Renamed(Result<(), std::io::Error>),
}

pub struct IoThread {
    requests_tx: Sender<(Id, IoReq)>,
    #[allow(clippy::type_complexity)]
    results: Arc<Mutex<(Receiver<(Id, IoOutcome)>, HashMap<Id, IoOutcome>)>>,
    task_id_generator: Arc<Mutex<IdGenerator>>,
}

impl IoThread {
    #[allow(clippy::too_many_lines)]
    pub fn get() -> &'static Self {
        static INSTANCE: LazyLock<IoThread> = LazyLock::new(|| {
            let (requests_tx, requests_rx) = channel::<(Id, IoReq)>();
            let (results_tx, results_rx) = channel::<(Id, IoOutcome)>();

            std::thread::Builder::new()
                .name("fs_io_thread".into())
                .spawn(move || {
                    let mut file_id_generator = IdGenerator::default();
                    let mut files = HashMap::new();
                    let mut read_buffer = vec![0_u8; 128];
                    
                    for (request_id, request) in requests_rx {
                        match request.options {
                            IoReqOptions::OpenFile(path) => {
                                let result = StdFile::open(path).map(|stdfile| {
                                    let id = file_id_generator.next();
                                    files.insert(id, stdfile);
                                    id
                                });

                                let _ = results_tx.send((request_id, IoOutcome::FileOpenedOrCreated(result)));
                                request.waker.wake();
                            },
                            IoReqOptions::CreateFile(path) => {
                                let result = StdFile::create(path).map(|stdfile| {
                                    let id = file_id_generator.next();
                                    files.insert(id, stdfile);
                                    id
                                });

                                let _ = results_tx.send((request_id, IoOutcome::FileOpenedOrCreated(result)));
                                request.waker.wake();
                            }
                            IoReqOptions::ReadFile(file, max_bytes) => {
                                if let Some(file) = files.get_mut(&file) {
                                    if read_buffer.len() < max_bytes {
                                        read_buffer.resize(max_bytes, 0);
                                    }
                                    
                                    let result = file.read(&mut read_buffer[0..max_bytes]).map(|n| {
                                        read_buffer[0..n].to_vec()
                                    });

                                    let _ = results_tx.send((request_id, IoOutcome::FileBytesRead(result)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::FullyReadFile(file) => {
                                if let Some(file) = files.get_mut(&file) {
                                    let mut stack_read_buffer = [0; 1024];
                                    
                                    let mut contents = vec![];
                                    let error = loop {
                                        match file.read(&mut stack_read_buffer) {
                                            Ok(0) => break None,
                                            Ok(n) => contents.extend_from_slice(&read_buffer[..n]),
                                            Err(e) => break Some(e),
                                        }
                                    };
                                    
                                    let result = error
                                        .map_or(
                                            Ok(contents),
                                            Err
                                        );
                                    
                                    let _ = results_tx.send((request_id, IoOutcome::FileBytesRead(result)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::WriteFile(file, buffer) => {
                                if let Some(file) = files.get_mut(&file) {
                                    let result = file.write(&buffer);
                                    let _ = results_tx.send((request_id, IoOutcome::FileBytesWritten(result)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::FileMetadata(file) => {
                                if let Some(file) = files.get_mut(&file) {
                                    let result = file.metadata();
                                    let _ = results_tx.send((request_id, IoOutcome::FileMetadata(result)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::FileToStd(file) => {
                                if let Some(file) = files.remove(&file) {
                                    let _ = results_tx.send((request_id, IoOutcome::FileToStd(file)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::SetFileLen(file, size) => {
                                if let Some(file) = files.get_mut(&file) {
                                    let res = file.set_len(size);
                                    let _ = results_tx.send((request_id, IoOutcome::FileLenSet(res)));
                                    request.waker.wake();
                                }
                            }
                            IoReqOptions::ChangeDir(path, change) => {
                                let res = match change {
                                    DirChange::CreateDir => std::fs::create_dir(path),
                                    DirChange::CreateDirAll => std::fs::create_dir_all(path),
                                    DirChange::RemoveDir => std::fs::remove_dir(path),
                                    DirChange::RemoveDirAll => std::fs::remove_dir_all(path),
                                };

                                let _ = results_tx.send((request_id, IoOutcome::DirChanged(res)));
                                request.waker.wake();
                            }
                            IoReqOptions::Copy(from, to) => {
                                let res = std::fs::copy(from, to);
                                let _ = results_tx.send((request_id, IoOutcome::Copied(res)));
                                request.waker.wake();
                            }
                            IoReqOptions::RemoveFile(path) => {
                                let res = std::fs::remove_file(path);
                                let _ = results_tx.send((request_id, IoOutcome::Removed(res)));
                                request.waker.wake();
                            }
                            IoReqOptions::Rename(from, to) => {
                                let res = std::fs::rename(from, to);
                                let _ = results_tx.send((request_id, IoOutcome::Renamed(res)));
                                request.waker.wake();
                            }
                        }
                    }
                })
                .expect("unable to spawn fs_io_thread");

            IoThread {
                requests_tx,
                results: Arc::new(Mutex::new((results_rx, HashMap::new()))),
                task_id_generator: Arc::new(Mutex::new(IdGenerator::default())),
            }
        });

        &INSTANCE
    }
    
    pub fn send_request (&self, options: IoReqOptions, waker: Waker) -> Id {
        let id = self.task_id_generator.lock().unwrap().next();
        let _ = self.requests_tx.send((id, IoReq {
            waker, options
        }));
        id
    }
    
    pub fn try_recv_result (&self, id: Id) -> Option<IoOutcome> {
        let (rx, map) = &mut *self.results.lock().unwrap();
        map.extend(rx.try_iter());
        map.remove(&id)
    }
}