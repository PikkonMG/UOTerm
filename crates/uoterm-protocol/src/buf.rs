use crate::error::{ProtocolError, Result};
use crate::types::Serial;

const VARIABLE_LEN_U16_MAX: usize = u16::MAX as usize;
const UTF16_UNIT_BYTES: usize = 2;

fn utf16le_units(raw: &[u8]) -> Vec<u16> {
    raw.chunks_exact(UTF16_UNIT_BYTES)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

pub struct PacketWriter {
    buf: Vec<u8>,
}

impl PacketWriter {
    pub fn new(id: u8) -> Self {
        Self { buf: vec![id] }
    }

    pub fn with_variable(id: u8) -> Self {
        Self {
            buf: vec![id, 0, 0],
        }
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    pub fn i8(&mut self, v: i8) -> &mut Self {
        self.buf.push(v as u8);
        self
    }

    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn i16(&mut self, v: i16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn serial(&mut self, s: Serial) -> &mut Self {
        self.u32(s.0)
    }

    pub fn bytes(&mut self, data: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(data);
        self
    }

    pub fn ascii_fixed(&mut self, text: &str, width: usize) -> &mut Self {
        let mut raw = text.as_bytes().to_vec();
        raw.truncate(width.saturating_sub(1));
        raw.resize(width, 0);
        self.buf.extend_from_slice(&raw);
        self
    }

    pub fn ascii_z(&mut self, text: &str) -> &mut Self {
        self.buf.extend_from_slice(text.as_bytes());
        self.buf.push(0);
        self
    }

    pub fn utf16be_z(&mut self, text: &str) -> &mut Self {
        for c in text.encode_utf16() {
            self.buf.extend_from_slice(&c.to_be_bytes());
        }
        self.buf.extend_from_slice(&0u16.to_be_bytes());
        self
    }

    pub fn pad(&mut self, n: usize) -> &mut Self {
        self.buf.resize(self.buf.len() + n, 0);
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    pub fn finish_variable(mut self) -> Result<Vec<u8>> {
        let len = self.buf.len();
        if len > VARIABLE_LEN_U16_MAX {
            return Err(ProtocolError::InvalidLength {
                id: self.buf.first().copied().unwrap_or(0),
                length: 0,
            });
        }
        let n = len as u16;
        self.buf[1..3].copy_from_slice(&n.to_be_bytes());
        Ok(self.buf)
    }
}

pub struct PacketReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn need(&self, n: usize) -> Result<()> {
        if self.remaining() < n {
            Err(ProtocolError::Truncated {
                needed: self.pos + n,
                had: self.data.len(),
            })
        } else {
            Ok(())
        }
    }

    pub fn u8(&mut self) -> Result<u8> {
        self.need(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16> {
        self.need(2)?;
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32> {
        self.need(4)?;
        let v = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    pub fn serial(&mut self) -> Result<Serial> {
        Ok(Serial(self.u32()?))
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.need(n)?;
        self.pos += n;
        Ok(())
    }

    pub fn rest(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        self.need(n)?;
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn ascii_fixed(&mut self, width: usize) -> Result<String> {
        self.need(width)?;
        let slice = &self.data[self.pos..self.pos + width];
        self.pos += width;
        let end = slice.iter().position(|&b| b == 0).unwrap_or(width);
        Ok(String::from_utf8_lossy(&slice[..end]).into_owned())
    }

    pub fn ascii_z(&mut self) -> Result<String> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        let s = String::from_utf8_lossy(&self.data[start..self.pos]).into_owned();
        if self.pos < self.data.len() {
            self.pos += 1;
        }
        Ok(s)
    }

    pub fn utf16be_z(&mut self) -> Result<String> {
        let mut units = Vec::new();
        loop {
            let u = self.u16()?;
            if u == 0 {
                break;
            }
            units.push(u);
        }
        String::from_utf16(&units).map_err(|_| ProtocolError::Encoding)
    }

    /// UTF-16 little-endian text of `len_bytes` bytes with no terminator.
    /// Object property list arguments carry their length in bytes, not units.
    pub fn utf16le_fixed(&mut self, len_bytes: usize) -> Result<String> {
        let raw = self.take(len_bytes)?;
        String::from_utf16(&utf16le_units(raw)).map_err(|_| ProtocolError::Encoding)
    }

    /// UTF-16 little-endian text that always consumes `units` code units but
    /// stops at the first NUL. The argument block of a `0xDF` buff opens with
    /// such a field, which the reference client reads as two code units.
    pub fn utf16le_fixed_z(&mut self, units: usize) -> Result<String> {
        let raw = self.take(units * UTF16_UNIT_BYTES)?;
        let text: Vec<u16> = utf16le_units(raw)
            .into_iter()
            .take_while(|&unit| unit != 0)
            .collect();
        String::from_utf16(&text).map_err(|_| ProtocolError::Encoding)
    }

    pub fn utf16le_z(&mut self) -> Result<String> {
        let mut units = Vec::new();
        loop {
            self.need(UTF16_UNIT_BYTES)?;
            let u = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += UTF16_UNIT_BYTES;
            if u == 0 {
                break;
            }
            units.push(u);
        }
        String::from_utf16(&units).map_err(|_| ProtocolError::Encoding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProtocolError;
    use crate::types::PKT_LOGIN_REQUEST;

    #[test]
    fn ascii_roundtrip() {
        let mut w = PacketWriter::new(PKT_LOGIN_REQUEST);
        w.ascii_fixed("player", 30).ascii_fixed("secret", 30).u8(0);
        let buf = w.finish();
        assert_eq!(buf.len(), 62);
        let mut r = PacketReader::new(&buf);
        assert_eq!(r.u8().unwrap(), PKT_LOGIN_REQUEST);
        assert_eq!(r.ascii_fixed(30).unwrap(), "player");
        assert_eq!(r.ascii_fixed(30).unwrap(), "secret");
    }

    #[test]
    fn finish_variable_rejects_payload_over_u16() {
        let mut w = PacketWriter::with_variable(PKT_LOGIN_REQUEST);
        w.pad(VARIABLE_LEN_U16_MAX.saturating_sub(1));
        match w.finish_variable() {
            Err(ProtocolError::InvalidLength {
                id: PKT_LOGIN_REQUEST,
                ..
            }) => {}
            other => panic!("{other:?}"),
        }
    }
}
