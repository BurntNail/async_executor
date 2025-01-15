use std::net::{TcpListener as StdTcpListener, TcpStream as StdTcpStream, ToSocketAddrs};
use std::future::Future;
use std::io::{ErrorKind, Read};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TcpStream {
    stdstream: StdTcpStream,
}

impl TcpStream {
    pub fn from_std (stdstream: StdTcpStream) -> Result<Self, std::io::Error> {
        stdstream.set_nonblocking(true)?;
        Ok(Self {
            stdstream,
        })
    }

    pub async fn read (&mut self, output: &mut [u8]) -> Result<usize, std::io::Error> {
        AsyncRead {
            listener: &mut self.stdstream,
            output,
        }.await
    }
}

struct AsyncRead<'file> {
    listener: &'file mut StdTcpStream,
    output: &'file mut [u8],
}

impl<'file> Future for AsyncRead<'file> {
    type Output = Result<usize, std::io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut internal_output = self.output.to_vec();
        let res = self.listener.read(&mut internal_output);
        match res {
            Ok(n) => {
                self.output[..n].copy_from_slice(&internal_output[..n]);
                Poll::Ready(Ok(n))
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            },
            
        }
    }
}


pub struct TcpListener {
    stdlistener: StdTcpListener
}

impl TcpListener {
    pub fn bind (addr: impl ToSocketAddrs) -> Result<Self, std::io::Error> {
        let stdlistener = StdTcpListener::bind(addr)?;
        stdlistener.set_nonblocking(true)?;
        
        Ok(Self {
            stdlistener
        })
    }
    
    pub async fn accept (&self) -> Result<TcpStream, std::io::Error> {
        TcpListenerAccept {
            stdlistener: &self.stdlistener
        }.await
    }
}

struct TcpListenerAccept<'listener> {
    stdlistener: &'listener StdTcpListener
}

impl Future for TcpListenerAccept<'_> {
    type Output = Result<TcpStream, std::io::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.stdlistener.accept() {
            Ok((s, _)) => Poll::Ready(TcpStream::from_std(s)),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            },
        }
    }
}

pub async fn fully_read_from_socket (addr: impl ToSocketAddrs) -> Result<Vec<u8>, std::io::Error> {
    println!("[tcp] binding");
    let listener = TcpListener::bind(addr)?;
    println!("[tcp] bound, waiting for stream");
    let mut stream = listener.accept().await?;
    println!("[tcp] got stream");
    
    let mut output = vec![];
    let mut tmp = [0_u8; 1];
    loop {
        println!("[tcp] waiting for input");
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => output.extend(&tmp[0..n]),
            Err(e) => return Err(e),
        }
        println!("[tcp] got input, next step");
    }
    println!("[tcp] done");
    
    Ok(output)
}