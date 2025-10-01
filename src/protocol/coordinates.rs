use crate::{
    data::Coordinate,
    protocol::{P2Decodable, P2Encodable},
    server::P2Write,
};

impl P2Encodable for Coordinate {
    async fn write_p2encoded(
        &self,
        connection: &mut (impl P2Write + Unpin),
    ) -> tokio::io::Result<()> {
        CoordI16(self.x).write_p2encoded(connection).await?;
        CoordI16(self.y).write_p2encoded(connection).await?;
        Ok(())
    }
}
impl P2Decodable for Coordinate {
    async fn read_p2encoded(
        connection: &mut (impl crate::server::P2Read + Unpin),
    ) -> tokio::io::Result<Option<Self>> {
        Ok(
            match (
                CoordI16::read_p2encoded(connection).await?,
                CoordI16::read_p2encoded(connection).await?,
            ) {
                (Some(CoordI16(x)), Some(CoordI16(y))) => Some(Self { x, y }),
                (None, _) | (_, None) => None,
            },
        )
    }
}

pub struct CoordI8(pub i8);

pub struct CoordI16(pub i16);

impl P2Encodable for CoordI16 {
    async fn write_p2encoded(
        &self,
        connection: &mut (impl P2Write + Unpin),
    ) -> tokio::io::Result<()> {
        if self.0.abs() <= 127 {
            connection.write_all(&[0]).await?;
            CoordI8(self.0 as i8).write_p2encoded(connection).await?;
        } else if self.0 >= 128 {
            CoordI8(((self.0 + 127) / 255) as i8)
                .write_p2encoded(connection)
                .await?;
            CoordI8(((self.0 + 127) % 255 - 127) as i8)
                .write_p2encoded(connection)
                .await?;
        } else if self.0 <= -128 {
            CoordI8(-(((self.0.abs() + 127) / 255) as i8))
                .write_p2encoded(connection)
                .await?;
            CoordI8(((self.0.abs() + 127) % 255 - 127) as i8)
                .write_p2encoded(connection)
                .await?;
        }
        Ok(())
    }
}
impl P2Decodable for CoordI16 {
    async fn read_p2encoded(
        connection: &mut (impl crate::server::P2Read + Unpin),
    ) -> tokio::io::Result<Option<Self>> {
        let bytes = match (
            CoordI8::read_p2encoded(connection).await?,
            CoordI8::read_p2encoded(connection).await?,
        ) {
            (Some(a), Some(b)) => (a, b),
            (None, _) | (_, None) => return Ok(None),
        };
        Ok(Some(if bytes.0.0 == 0 {
            Self(bytes.1.0 as i16)
        } else if bytes.0.0 > 0 {
            Self((bytes.0.0 as i16 * 255) + (bytes.1.0 as i16 + 127) - 127)
        } else {
            Self(-((bytes.0.0.abs() as i16 * 255) + (bytes.1.0 as i16 + 127) - 127))
        }))
    }
}

impl P2Encodable for CoordI8 {
    async fn write_p2encoded(
        &self,
        connection: &mut (impl P2Write + Unpin),
    ) -> tokio::io::Result<()> {
        connection
            .write_all(&[if self.0 >= 0 {
                self.0 as u8
            } else {
                (self.0.abs() - 1) as u8 | 0b10000000
            }])
            .await?;
        Ok(())
    }
}
impl P2Decodable for CoordI8 {
    async fn read_p2encoded(
        connection: &mut (impl crate::server::P2Read + Unpin),
    ) -> tokio::io::Result<Option<Self>> {
        let mut byte = [0u8; 1];
        connection.read_exact(&mut byte).await?;
        let byte = byte[0];
        Ok(if byte != 0xFF {
            Some(if byte & 0b10000000 == 0 {
                Self(byte as i8)
            } else {
                Self(-((byte & 0b01111111) as i8 + 1))
            })
        } else {
            None
        })
    }
}

#[tokio::test]
async fn test_coord_encoding_and_decoding() {
    let mut connection = super::enc_dec::TestLoopbackConnection::default();
    for n in -127..=127 {
        CoordI8(n).write_p2encoded(&mut connection).await.unwrap();
        let m = CoordI8::read_p2encoded(&mut connection)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(n, m.0);
    }
    for n in -32512..=32512 {
        CoordI16(n).write_p2encoded(&mut connection).await.unwrap();
        let m = CoordI16::read_p2encoded(&mut connection)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(n, m.0);
    }
}
