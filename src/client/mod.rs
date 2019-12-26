use hex::FromHex;
use hyper::{
    client::connect::{Connected, Connection},
    service::Service,
    Uri,
};
use pin_project::pin_project;
use std::{
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

#[pin_project]
#[derive(Debug)]
pub struct UnixStream {
    #[pin]
    unix_stream: tokio::net::UnixStream,
}

impl UnixStream {
    async fn connect<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let unix_stream = tokio::net::UnixStream::connect(path).await?;
        Ok(Self { unix_stream })
    }
}

impl tokio::io::AsyncWrite for UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().unix_stream.poll_write(cx, buf)
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().unix_stream.poll_flush(cx)
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().unix_stream.poll_shutdown(cx)
    }
}

impl tokio::io::AsyncRead for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().unix_stream.poll_read(cx, buf)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnixConnector;

impl Unpin for UnixConnector {}

impl Service<Uri> for UnixConnector {
    type Response = UnixStream;
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;
    fn call(
        &mut self,
        req: Uri,
    ) -> Self::Future {
        let fut = async move {
            let path = parse_socket_path(req)?;
            UnixStream::connect(path).await
        };

        Box::pin(fut)
    }
    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Connection for UnixStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

fn parse_socket_path(uri: Uri) -> Result<std::path::PathBuf, io::Error> {
    if uri.scheme_str() != Some("unix") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid URL, scheme must be unix",
        ));
    }

    if let Some(host) = uri.host() {
        let bytes = Vec::from_hex(host).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid URL, host must be a hex-encoded path",
            )
        })?;

        Ok(PathBuf::from(String::from_utf8_lossy(&bytes).into_owned()))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid URL, host must be present",
        ))
    }
}
