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
        struct AsyncRead<'a> {
            listener: &'a mut StdTcpStream,
            output: &'a mut [u8],
        }

        impl Future for AsyncRead<'_> {
            type Output = Result<usize, std::io::Error>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut internal_output = self.output.to_vec(); //TODO: don't allocate a new buffer on every read?
                //could use a [0; self.output.len()], but that could stack overflow i think
                let res = self.listener.read(&mut internal_output);
                match res {
                    Ok(n) => {
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
        
        AsyncRead {
            listener: &mut self.stdstream,
            output,
        }.await
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

        TcpListenerAccept {
            stdlistener: &self.stdlistener
        }.await
    }
}

pub async fn fully_read_from_socket (addr: impl ToSocketAddrs + Send) -> Result<Vec<u8>, std::io::Error> {
    let listener = TcpListener::bind(addr)?;
    let mut stream = listener.accept().await?;

    let mut output = vec![];
    let mut tmp = [0_u8; 1];
    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => output.extend(&tmp[0..n]),
            Err(e) => return Err(e),
        }
    }
    println!("[tcp] done");
    
    Ok(output)
}