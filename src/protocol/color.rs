use crate::{
    data::Color,
    protocol::{P2Decodable, P2Encodable, coordinates::CoordI16},
    server::P2Write,
};

impl P2Encodable for Color {
    async fn write_p2encoded(
        &self,
        connection: &mut (impl P2Write + Unpin),
    ) -> tokio::io::Result<()> {
        if (self.r & 0b10000) == 0 {
            CoordI16(
                1 + (((self.r as i16 & 0b1111) << 10)
                    | ((self.g as i16 & 0b11111) << 5)
                    | (self.b as i16 & 0b11111)),
            )
        } else {
            CoordI16(
                -1 - (((self.r as i16 & 0b1111) << 10)
                    | ((self.g as i16 & 0b11111) << 5)
                    | (self.b as i16 & 0b11111)),
            )
        }
        .write_p2encoded(connection)
        .await?;
        Ok(())
    }
}

impl P2Decodable for Color {
    async fn read_p2encoded(
        connection: &mut (impl crate::server::P2Read + Unpin),
    ) -> tokio::io::Result<Option<Self>> {
        let CoordI16(y) = if let Some(y) = CoordI16::read_p2encoded(connection).await? {
            y
        } else {
            return Ok(None);
        };
        let x = if y > 0 {
            y as u32 - 1
        } else if y < 0 {
            y.abs() as u32 - 1 + 16384
        } else {
            return Ok(None);
        };
        Ok(Some(Self {
            r: ((x & 0b111110000000000) >> 10) as u8,
            g: ((x & 0b000001111100000) >> 5) as u8,
            b: (x & 0b000000000011111) as u8,
        }))
    }
}

#[tokio::test]
async fn test_color_encoding_and_decoding() {
    let mut connection = super::enc_dec::TestLoopbackConnection::default();
    for r in 0..32 {
        for g in 0..32 {
            for b in 0..32 {
                Color { r, g, b }
                    .write_p2encoded(&mut connection)
                    .await
                    .unwrap();
                let Color { r: x, g: y, b: z } = Color::read_p2encoded(&mut connection)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!((r, g, b), (x, y, z));
            }
        }
    }
}
