use crate::server::{P2Read, P2Write};

pub trait P2Encodable {
    async fn write_p2encoded(
        &self,
        connection: &mut (impl P2Write + Unpin),
    ) -> tokio::io::Result<()>;
}

pub trait P2Decodable: Sized {
    async fn read_p2encoded(
        connection: &mut (impl P2Read + Unpin),
    ) -> tokio::io::Result<Option<Self>>;
}

impl P2Write for Vec<u8> {
    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        self.extend(buf);
        Ok(())
    }
    async fn flush(&mut self) -> tokio::io::Result<()> {
        Ok(())
    }
    async fn close(&mut self) -> tokio::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct TestLoopbackConnection(std::collections::VecDeque<u8>);
#[cfg(test)]
impl P2Write for TestLoopbackConnection {
    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        self.0.extend(buf);
        Ok(())
    }
    async fn flush(&mut self) -> tokio::io::Result<()> {
        Ok(())
    }
    async fn close(&mut self) -> tokio::io::Result<()> {
        Ok(())
    }
}
#[cfg(test)]
impl P2Read for TestLoopbackConnection {
    async fn read_exact(&mut self, buf: &mut [u8]) -> tokio::io::Result<()> {
        let data = self.0.make_contiguous();
        let len = buf.len().min(data.len());
        buf[0..len].copy_from_slice(&data[0..len]);
        self.0.drain(0..len);
        Ok(())
    }
}
