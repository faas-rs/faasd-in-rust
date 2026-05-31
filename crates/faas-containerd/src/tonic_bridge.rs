//! Bridge adapters: run tonic gRPC client on asupersync runtime.
//!
//! tonic 0.12 uses hyper 1.x which abstracts over runtime via three traits:
//! - `hyper::rt::Executor` — spawns background tasks
//! - `hyper::rt::Read + Write` — async I/O
//!
//! This module provides implementations backed by asupersync, eliminating
//! any need for a tokio runtime.

use asupersync::io::{AsyncRead, AsyncWrite, ReadBuf};
use asupersync::net::unix::UnixStream;
use asupersync::runtime::RuntimeHandle;
use http::Uri;
use hyper::rt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

// ── hyper::rt::Executor ─────────────────────────────────────────────────

/// Implements `hyper::rt::Executor` by forwarding to `RuntimeHandle::spawn`.
///
/// tonic calls this to spawn background HTTP/2 connection management tasks.
/// All spawned futures run on the asupersync runtime.
#[derive(Clone)]
pub struct AsupersyncExecutor {
    pub handle: RuntimeHandle,
}

impl<F> rt::Executor<F> for AsupersyncExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        self.handle.spawn(fut);
    }
}

// ── hyper::rt::Read + Write adapter ─────────────────────────────────────

/// Wraps an asupersync `AsyncRead + AsyncWrite + Unpin` type to implement
/// `hyper::rt::Read + Write`.
///
/// For reads: uses a stack-local bounce buffer to bridge asupersync's `ReadBuf`
/// and hyper's `ReadBufCursor`. Capped at 8 KiB to limit stack usage.
pub struct HyperAdapter<T> {
    pub inner: T,
}

impl<T: AsyncRead + Unpin> rt::Read for HyperAdapter<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let remaining = buf.remaining();
        if remaining == 0 {
            return Poll::Ready(Ok(()));
        }
        let cap = remaining.min(8192);
        let mut tmp = vec![0u8; cap];
        let mut asupersync_buf = ReadBuf::new(&mut tmp);
        match AsyncRead::poll_read(Pin::new(&mut self.inner), cx, &mut asupersync_buf) {
            Poll::Ready(Ok(())) => {
                let n = asupersync_buf.filled().len();
                if n == 0 {
                    return Poll::Ready(Ok(()));
                }
                buf.put_slice(asupersync_buf.filled());
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: AsyncWrite + Unpin> rt::Write for HyperAdapter<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        AsyncWrite::poll_write(Pin::new(&mut self.inner), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.inner), cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.inner), cx)
    }
}

// ── Unix socket connector (tower::Service<Uri>) ────────────────────────

/// A `tower::Service` that connects to a fixed Unix domain socket path.
///
/// Used with `tonic::transport::Endpoint::connect_with_connector_lazy()`.
#[derive(Clone)]
pub struct UnixConnector {
    pub path: String,
}

impl tower::Service<Uri> for UnixConnector {
    type Response = HyperAdapter<UnixStream>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _uri: Uri) -> Self::Future {
        let path = self.path.clone();
        Box::pin(async move {
            let stream = UnixStream::connect(&path).await?;
            Ok(HyperAdapter { inner: stream })
        })
    }
}

// ── Convenience: build a tonic Channel ──────────────────────────────────

/// Build a tonic `Channel` connected to a Unix socket, using asupersync
/// for all I/O and task spawning.
pub fn connect_channel(handle: RuntimeHandle, socket_path: &str) -> tonic::transport::Channel {
    tonic::transport::Endpoint::from_static("http://containerd")
        .executor(AsupersyncExecutor { handle })
        .connect_with_connector_lazy(UnixConnector {
            path: socket_path.to_string(),
        })
}
