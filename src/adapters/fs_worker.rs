use std::collections::HashMap;
use std::fs::File as StdFile;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::task::Waker;
use std::thread::JoinHandle;
use crate::id::{AtomicIdGenerator, Id, IdGenerator};

pub struct FileThreadRequest {
    waker: Waker,
    options: FileThreadRequestOptions
}

pub enum FileThreadRequestOptions {
    OpenFile(PathBuf),
    CreateFile(PathBuf),
    Read(Id, usize),
    Write(Id, Vec<u8>),
}

pub enum FileThreadResult {
    FileOpenedOrCreated(Result<Id, std::io::Error>),
    BytesRead(Result<Vec<u8>, std::io::Error>),
    BytesWritten(Result<usize, std::io::Error>)
}

pub struct FileThread {
    handle: JoinHandle<()>,
    requests_tx: Sender<(Id, FileThreadRequest)>,
    results: Arc<Mutex<(Receiver<(Id, FileThreadResult)>, HashMap<Id, FileThreadResult>)>>,
    task_id_generator: AtomicIdGenerator,
}

impl FileThread {
    pub fn get() -> &'static Self {
        static INSTANCE: LazyLock<FileThread> = LazyLock::new(|| {
            let (requests_tx, requests_rx) = channel::<(Id, FileThreadRequest)>();
            let (results_tx, results_rx) = channel::<(Id, FileThreadResult)>();

            let handle = std::thread::Builder::new()
                .name("fs_io_thread".into())
                .spawn(move || {
                    let mut file_id_generator = IdGenerator::default();
                    let mut files = HashMap::new();
                    let mut read_buffer = vec![0_u8; 128];
                    
                    for (request_id, request) in requests_rx.into_iter() {
                        match request.options {
                            FileThreadRequestOptions::OpenFile(path) => {
                                let result = StdFile::open(path).map(|stdfile| {
                                    let id = file_id_generator.next().expect("no more file IDs left");
                                    files.insert(id, stdfile);
                                    id
                                });

                                let _ = results_tx.send((request_id, FileThreadResult::FileOpenedOrCreated(result)));
                                request.waker.wake();
                            },
                            FileThreadRequestOptions::CreateFile(path) => {
                                let result = StdFile::create(path).map(|stdfile| {
                                    let id = file_id_generator.next().expect("no more file IDs left");
                                    files.insert(id, stdfile);
                                    id
                                });

                                let _ = results_tx.send((request_id, FileThreadResult::FileOpenedOrCreated(result)));
                                request.waker.wake();
                            }
                            FileThreadRequestOptions::Read(file, max_bytes) => {
                                if let Some(file) = files.get_mut(&file) {
                                    if read_buffer.len() < max_bytes {
                                        read_buffer.resize(max_bytes, 0);
                                    }
                                    
                                    
                                    let result = file.read(&mut read_buffer[0..max_bytes]).map(|n| {
                                        (&read_buffer[0..n]).to_vec()
                                    });

                                    let _ = results_tx.send((request_id, FileThreadResult::BytesRead(result)));
                                    request.waker.wake();
                                }
                            }
                            FileThreadRequestOptions::Write(file, buffer) => {
                                if let Some(file) = files.get_mut(&file) {
                                    let result = file.write(&buffer);
                                    let _ = results_tx.send((request_id, FileThreadResult::BytesWritten(result)));
                                    request.waker.wake();
                                }
                            }
                        }
                    }
                })
                .expect("unable to spawn fs_io_thread");

            FileThread {
                handle,
                requests_tx,
                results: Arc::new(Mutex::new((results_rx, HashMap::new()))),
                task_id_generator: AtomicIdGenerator::default(),
            }
        });

        &INSTANCE
    }
    
    pub fn send_request (&self, options: FileThreadRequestOptions, waker: Waker) -> Id {
        let id = self.task_id_generator.next();
        let _ = self.requests_tx.send((id, FileThreadRequest {
            waker, options
        }));
        id
    }
    
    pub fn try_recv_result (&self, id: &Id) -> Option<FileThreadResult> {
        let (rx, map) = &mut *self.results.lock().unwrap();
        map.extend(rx.try_iter());
        map.remove(id)
    }
}