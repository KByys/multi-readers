use crate::{BytesReader, SliceReader};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

macro_rules! ready {
    ($n:expr) => {
        if ($n == 0) {
            return Poll::Ready(Ok(()))
        }
    };
    ($v1:expr, $v2:expr) => {
        if ($v1 == $v2) {
            return Poll::Ready(Ok(()))
        }
    }
}
impl AsyncRead for BytesReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut slice_reader = self.as_slice_reader();
        let result = tokio::io::AsyncRead::poll_read(Pin::new(&mut slice_reader), _cx, buf);
        self.get_mut().pos = slice_reader.pos;
        result
    }
}
impl<'a> AsyncRead for SliceReader<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        ready!(self.pos, self.buf.len());
        ready!(buf.capacity());
        let amt = std::cmp::min(self.buf.len() - self.pos, buf.capacity());
        let slice =&self.buf[self.pos..self.pos + amt]; 
        buf.put_slice(slice);
        self.get_mut().pos += amt;
        Poll::Ready(Ok(()))
    }
}


pub struct AsyncMultiReaders<'iter, 'life> {
    current: Option<Box<dyn AsyncRead + Unpin + 'life>>,
    iter: Box<dyn Iterator<Item = Box<dyn AsyncRead + Unpin + 'life>> + 'iter>,
    buf: Vec<u8>,
    filled: usize,
}

impl<'iter, 'life> AsyncMultiReaders<'iter, 'life> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter(
        iter: impl Iterator<Item = Box<dyn AsyncRead + Unpin + 'life>> + 'iter,
    ) -> Self {
        Self {
            current: None,
            iter: Box::new(iter),
            buf: Vec::new(),
            filled: 0
        }
    }
}

impl<'iter, 'life> AsyncRead for AsyncMultiReaders<'iter, 'life> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        ready!(buf.capacity());
        let this = self.get_mut();
        if this.filled == 0 {
            buf.clear();
            this.buf = vec![0; buf.capacity()];
        }

        while this.filled < buf.capacity() {
            if this.current.is_none() {
                this.current = this.iter.next();
            }
            match &mut this.current {
                Some(r) => {
                    let slice = &mut this.buf[this.filled..];
                    let mut tmp = ReadBuf::new(slice);
                    match Pin::new(r).poll_read(cx, &mut tmp) {
                        Poll::Ready(Ok(_)) => {
                            // Read EOF
                            this.filled += tmp.filled().len();
                            if tmp.filled().len() < buf.capacity() - this.filled {
                                this.current = None;
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            buf.put_slice(tmp.filled());
                            return Poll::Ready(Err(e));
                        }
                        Poll::Pending => {
                            return Poll::Pending
                        }
                    }
                }
                // EOF
                _ => break
            }
        }
        buf.put_slice(&this.buf[..this.filled]);
        this.buf = Vec::new();
        this.filled = 0;
        Poll::Ready(Ok(()))
    }
}
