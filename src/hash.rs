use blake3::{Hash, Hasher};
use std::io::{Read, Write};

pub struct Blake3Reader<R: Read> {
    inner: R,
    hasher: Hasher,
}

impl<R: Read> Blake3Reader<R> {
    pub fn new(inner: R) -> Blake3Reader<R> {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    pub fn finalize(self) -> (R, Hash) {
        (self.inner, self.hasher.finalize())
    }
}

impl<R: Read> Read for Blake3Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            Ok(0)
        } else {
            self.hasher.update(&buf[..n]);
            Ok(n)
        }
    }
}

pub struct Blake3Writer<W: Write> {
    inner: W,
    hasher: Hasher,
}

impl<W: Write> Blake3Writer<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    pub fn finalize(self) -> (W, Hash) {
        (self.inner, self.hasher.finalize())
    }
}

impl<W: Write> Write for Blake3Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()?;
        self.hasher.flush()
    }
}
