pub trait P2Read {
    async fn read_exact(&mut self, buf: &mut [u8]) -> tokio::io::Result<()>;
}
pub trait P2Write: Send + 'static {
    // async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = tokio::io::Result<()>> + Send;
    // async fn flush(&mut self) -> tokio::io::Result<()>;
    fn flush(&mut self) -> impl Future<Output = tokio::io::Result<()>> + Send;
    // async fn close(&mut self) -> tokio::io::Result<()>;
    fn close(&mut self) -> impl Future<Output = tokio::io::Result<()>> + Send;
}
