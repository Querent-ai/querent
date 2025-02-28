use bytes::Bytes;
use futures::Stream;
use pin_project::pin_project;
use std::{
	io,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::io::AsyncRead;

#[pin_project]
pub struct BytesStreamReader<S>
where
	S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin,
{
	#[pin]
	stream: S,
	buffer: Option<Bytes>,
}

impl<S> BytesStreamReader<S>
where
	S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin,
{
	pub fn new(stream: S) -> Self {
		BytesStreamReader { stream, buffer: None }
	}
}

impl<S> AsyncRead for BytesStreamReader<S>
where
	S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin,
{
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		let this = self.project();

		// If we have data in the buffer, use it
		if let Some(buffer) = this.buffer {
			let bytes_to_copy = std::cmp::min(buffer.len(), buf.remaining());
			buf.put_slice(&buffer[..bytes_to_copy]);

			if bytes_to_copy < buffer.len() {
				// We have leftover data
				*this.buffer = Some(buffer.slice(bytes_to_copy..));
			} else {
				// We've used all the data
				*this.buffer = None;
			}

			return Poll::Ready(Ok(()));
		}

		// Try to get more data from the stream
		match this.stream.poll_next(cx) {
			Poll::Ready(Some(Ok(bytes))) => {
				let bytes_to_copy = std::cmp::min(bytes.len(), buf.remaining());
				buf.put_slice(&bytes[..bytes_to_copy]);

				if bytes_to_copy < bytes.len() {
					// We have leftover data
					*this.buffer = Some(bytes.slice(bytes_to_copy..));
				}

				Poll::Ready(Ok(()))
			},
			Poll::Ready(Some(Err(e))) => Poll::Ready(Err(io::Error::new(
				io::ErrorKind::Other,
				format!("Request error: {}", e),
			))),
			Poll::Ready(None) => {
				// End of stream
				Poll::Ready(Ok(()))
			},
			Poll::Pending => Poll::Pending,
		}
	}
}
