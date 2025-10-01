pub trait P2Read {
    async fn read_exact(&mut self, buf: &mut [u8]) -> tokio::io::Result<()>;
}
pub trait P2Write {
    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()>;
    async fn flush(&mut self) -> tokio::io::Result<()>;
    async fn close(&mut self) -> tokio::io::Result<()>;
}
