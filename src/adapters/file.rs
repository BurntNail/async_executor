use std::path::{Path, PathBuf};
use crate::id::Id;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::adapters::fs_worker::{FileThread, FileThreadRequestOptions, FileThreadResult};

pub struct File {
    id: Id,
}

impl File {
    pub async fn open (p: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        enum FileOpenState {
            Open(PathBuf),
            Waiting(Id),
            Done
        }
        struct FileOpen(FileOpenState);

        impl Future for FileOpen {
            type Output = Result<File, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = FileThread::get();

                match std::mem::replace(&mut self.0, FileOpenState::Done) {
                    FileOpenState::Open(pathbuf) => {
                        let id = thread.send_request(FileThreadRequestOptions::OpenFile(pathbuf), cx.waker().clone());
                        self.0 = FileOpenState::Waiting(id);
                        Poll::Pending
                    }
                    FileOpenState::Waiting(id) => {
                        match thread.try_recv_result(&id) {
                            None => {
                                self.0 = FileOpenState::Waiting(id);
                                Poll::Pending
                            },
                            Some(result) => {
                                if let FileThreadResult::FileOpenedOrCreated(res) = result {
                                    Poll::Ready(res.map(|id| File {id}))
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    FileOpenState::Done => panic!("tried to poll file after completion")
                }
            }
        }

        FileOpen(FileOpenState::Open(p.as_ref().to_path_buf())).await
    }

    pub async fn create (path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        enum FileCreateState {
            Create(PathBuf),
            Waiting(Id),
            Done
        }
        struct FileCreate(FileCreateState);

        impl Future for FileCreate {
            type Output = Result<File, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = FileThread::get();

                match std::mem::replace(&mut self.0, FileCreateState::Done) {
                    FileCreateState::Create(pathbuf) => {
                        let id = thread.send_request(FileThreadRequestOptions::CreateFile(pathbuf), cx.waker().clone());
                        self.0 = FileCreateState::Waiting(id);
                        Poll::Pending
                    }
                    FileCreateState::Waiting(id) => {
                        match thread.try_recv_result(&id) {
                            None => {
                                self.0 = FileCreateState::Waiting(id);
                                Poll::Pending
                            },
                            Some(result) => {
                                if let FileThreadResult::FileOpenedOrCreated(res) = result {
                                    Poll::Ready(res.map(|id| File {id}))
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    FileCreateState::Done => panic!("tried to poll file after completion")
                }
            }
        }

        FileCreate(FileCreateState::Create(path.as_ref().to_path_buf())).await
    }

    pub async fn read (&mut self, max_bytes: usize) -> Result<Vec<u8>, std::io::Error> {
        enum FileReadState {
            Read(Id, usize),
            Waiting(Id),
            Done
        }
        struct FileRead(FileReadState);

        impl Future for FileRead {
            type Output = Result<Vec<u8>, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = FileThread::get();

                match std::mem::replace(&mut self.0, FileReadState::Done) {
                    FileReadState::Read(max_bytes, file_id) => {
                        let task_id = thread.send_request(FileThreadRequestOptions::Read(max_bytes, file_id), cx.waker().clone());
                        self.0 = FileReadState::Waiting(task_id);
                        Poll::Pending
                    }
                    FileReadState::Waiting(task_id) => {
                        match thread.try_recv_result(&task_id) {
                            None => {
                                self.0 = FileReadState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(result) => {
                                if let FileThreadResult::BytesRead(res) = result {
                                    Poll::Ready(res)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    FileReadState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }

        FileRead(FileReadState::Read(self.id, max_bytes)).await
    }

    pub async fn write (&mut self, buf: Vec<u8>) -> Result<usize, std::io::Error> {
        enum FileWriteState {
            Write(Id, Vec<u8>),
            Waiting(Id),
            Done
        }
        struct FileWrite(FileWriteState);

        impl Future for FileWrite {
            type Output = Result<usize, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let ft = FileThread::get();

                match std::mem::replace(&mut self.0, FileWriteState::Done) {
                    FileWriteState::Write(id, buf) => {
                        let task_id = ft.send_request(FileThreadRequestOptions::Write(id, buf), cx.waker().clone());
                        self.0 = FileWriteState::Waiting(task_id);
                        Poll::Pending
                    }
                    FileWriteState::Waiting(id) => {
                        match ft.try_recv_result(&id) {
                            None => {
                                self.0 = FileWriteState::Waiting(id);
                                Poll::Pending
                            }
                            Some(res) => {
                                if let FileThreadResult::BytesWritten(res) = res {
                                    Poll::Ready(res)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    FileWriteState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }

        FileWrite(FileWriteState::Write(self.id, buf)).await
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
}