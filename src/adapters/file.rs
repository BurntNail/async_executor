use std::path::{Path, PathBuf};
use crate::id::Id;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::adapters::io_worker::{IoThread, IoReqOptions, IoOutcome};

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
                let thread = IoThread::get();

                match std::mem::replace(&mut self.0, FileOpenState::Done) {
                    FileOpenState::Open(pathbuf) => {
                        let id = thread.send_request(IoReqOptions::OpenFile(pathbuf), cx.waker().clone());
                        self.0 = FileOpenState::Waiting(id);
                        Poll::Pending
                    }
                    FileOpenState::Waiting(id) => {
                        match thread.try_recv_result(id) {
                            None => {
                                self.0 = FileOpenState::Waiting(id);
                                Poll::Pending
                            },
                            Some(result) => {
                                if let IoOutcome::FileOpenedOrCreated(res) = result {
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
                let thread = IoThread::get();

                match std::mem::replace(&mut self.0, FileCreateState::Done) {
                    FileCreateState::Create(pathbuf) => {
                        let id = thread.send_request(IoReqOptions::CreateFile(pathbuf), cx.waker().clone());
                        self.0 = FileCreateState::Waiting(id);
                        Poll::Pending
                    }
                    FileCreateState::Waiting(id) => {
                        match thread.try_recv_result(id) {
                            None => {
                                self.0 = FileCreateState::Waiting(id);
                                Poll::Pending
                            },
                            Some(result) => {
                                if let IoOutcome::FileOpenedOrCreated(res) = result {
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
                let thread = IoThread::get();

                match std::mem::replace(&mut self.0, FileReadState::Done) {
                    FileReadState::Read(max_bytes, file_id) => {
                        let task_id = thread.send_request(IoReqOptions::ReadFile(max_bytes, file_id), cx.waker().clone());
                        self.0 = FileReadState::Waiting(task_id);
                        Poll::Pending
                    }
                    FileReadState::Waiting(task_id) => {
                        match thread.try_recv_result(task_id) {
                            None => {
                                self.0 = FileReadState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(result) => {
                                if let IoOutcome::FileBytesRead(res) = result {
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

    pub async fn read_to_end (&mut self) -> Result<Vec<u8>, std::io::Error> {
        enum FullyReadState {
            FullyRead(Id),
            Waiting(Id),
            Done
        }
        struct FullyRead(FullyReadState);

        impl Future for FullyRead {
            type Output = Result<Vec<u8>, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = IoThread::get();
                match std::mem::replace(&mut self.0, FullyReadState::Done) {
                    FullyReadState::FullyRead(file_id) => {
                        let task_id = thread.send_request(IoReqOptions::FullyReadFile(file_id), cx.waker().clone());
                        self.0 = FullyReadState::Waiting(task_id);
                        Poll::Pending
                    }
                    FullyReadState::Waiting(task_id) => {
                        match thread.try_recv_result(task_id) {
                            None => {
                                self.0 = FullyReadState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(result) => {
                                if let IoOutcome::FileBytesRead(res) = result {
                                    Poll::Ready(res)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    FullyReadState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }

        FullyRead(FullyReadState::FullyRead(self.id)).await
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
                let thread = IoThread::get();

                match std::mem::replace(&mut self.0, FileWriteState::Done) {
                    FileWriteState::Write(id, buf) => {
                        let task_id = thread.send_request(IoReqOptions::WriteFile(id, buf), cx.waker().clone());
                        self.0 = FileWriteState::Waiting(task_id);
                        Poll::Pending
                    }
                    FileWriteState::Waiting(id) => {
                        match thread.try_recv_result(id) {
                            None => {
                                self.0 = FileWriteState::Waiting(id);
                                Poll::Pending
                            }
                            Some(res) => {
                                if let IoOutcome::FileBytesWritten(res) = res {
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

    pub async fn metadata (&self) -> Result<std::fs::Metadata, std::io::Error> {
        enum MetadataState {
            Metadata(Id),
            Waiting(Id),
            Done
        }
        struct Metadata(MetadataState);

        impl Future for Metadata {
            type Output = Result<std::fs::Metadata, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = IoThread::get();
                
                match std::mem::replace(&mut self.0, MetadataState::Done) {
                    MetadataState::Metadata(file_id) => {
                        let task_id = thread.send_request(IoReqOptions::FileMetadata(file_id), cx.waker().clone());
                        self.0 = MetadataState::Waiting(task_id);
                        Poll::Pending
                    }
                    MetadataState::Waiting(task_id) => {
                        match thread.try_recv_result(task_id) {
                            None => {
                                self.0 = MetadataState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(res) => {
                                if let IoOutcome::FileMetadata(res) = res {
                                    Poll::Ready(res)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    MetadataState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }
        
        Metadata(MetadataState::Metadata(self.id)).await
    }

    pub async fn to_std (self) -> std::fs::File {
        enum ToStdState {
            ToStd(Id),
            Waiting(Id),
            Done
        }
        struct ToStd (ToStdState);

        impl Future for ToStd {
            type Output = std::fs::File;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = IoThread::get();
                match std::mem::replace(&mut self.0, ToStdState::Done) {
                    ToStdState::ToStd(file_id) => {
                        let task_id = thread.send_request(IoReqOptions::FileToStd(file_id), cx.waker().clone());
                        self.0 = ToStdState::Waiting(task_id);
                        Poll::Pending
                    }
                    ToStdState::Waiting(task_id) => {
                        match thread.try_recv_result(task_id) {
                            None => {
                                self.0 = ToStdState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(res) => {
                                if let IoOutcome::FileToStd(file) = res {
                                    Poll::Ready(file)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    ToStdState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }

        ToStd(ToStdState::ToStd(self.id)).await
    }

    pub async fn set_len (&mut self, size: u64) -> Result<(), std::io::Error> {
        enum SetLenState {
            SetLen(Id, u64),
            Waiting(Id),
            Done
        }
        struct SetLen(SetLenState);

        impl Future for SetLen {
            type Output = Result<(), std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let thread = IoThread::get();
                match std::mem::replace(&mut self.0, SetLenState::Done) {
                    SetLenState::SetLen(file_id, size) => {
                        let task_id = thread.send_request(IoReqOptions::SetFileLen(file_id, size), cx.waker().clone());
                        self.0 = SetLenState::Waiting(task_id);
                        Poll::Pending
                    }
                    SetLenState::Waiting(task_id) => {
                        match thread.try_recv_result(task_id) {
                            None => {
                                self.0 = SetLenState::Waiting(task_id);
                                Poll::Pending
                            }
                            Some(res) => {
                                if let IoOutcome::FileLenSet(res) = res {
                                    Poll::Ready(res)
                                } else {
                                    panic!("found incorrect type from file thread")
                                }
                            }
                        }
                    }
                    SetLenState::Done => {
                        panic!("tried to poll file after completion")
                    }
                }
            }
        }


        SetLen(SetLenState::SetLen(self.id, size)).await
    }
}