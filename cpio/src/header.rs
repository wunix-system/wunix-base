// Структура заголовка формата SVR4 portable cpio (newc / 070701).

pub const CPIO_NEWC_MAGIC: &[u8; 6] = b"070701";
pub const CPIO_HEADER_SIZE: usize = 110;
pub const CPIO_TRAILER: &[u8] = b"TRAILER!!!";

#[derive(Clone, Debug, Default)]
pub struct CpioHeader {
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub mtime: u32,
    pub filesize: u32,
    pub devmajor: u32,
    pub devminor: u32,
    pub rdevmajor: u32,
    pub rdevminor: u32,
    pub namesize: u32,
    pub check: u32,
}

pub fn parse_hex_8(bytes: &[u8]) -> u32 {
    let mut val = 0u32;
    for &b in bytes {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        };
        val = (val << 4) | (digit as u32);
    }
    val
}

pub fn write_hex_8(buf: &mut [u8], mut val: u32) {
    let hex_chars = b"0123456789abcdef";
    for i in (0..8).rev() {
        buf[i] = hex_chars[(val & 0xF) as usize];
        val >>= 4;
    }
}

impl CpioHeader {
    pub fn parse(buf: &[u8; CPIO_HEADER_SIZE]) -> Option<Self> {
        if &buf[0..6] != CPIO_NEWC_MAGIC {
            return None;
        }

        Some(Self {
            ino: parse_hex_8(&buf[6..14]),
            mode: parse_hex_8(&buf[14..22]),
            uid: parse_hex_8(&buf[22..30]),
            gid: parse_hex_8(&buf[30..38]),
            nlink: parse_hex_8(&buf[38..46]),
            mtime: parse_hex_8(&buf[46..54]),
            filesize: parse_hex_8(&buf[54..62]),
            devmajor: parse_hex_8(&buf[62..70]),
            devminor: parse_hex_8(&buf[70..78]),
            rdevmajor: parse_hex_8(&buf[78..86]),
            rdevminor: parse_hex_8(&buf[86..94]),
            namesize: parse_hex_8(&buf[94..102]),
            check: parse_hex_8(&buf[102..110]),
        })
    }

    pub fn serialize(&self, buf: &mut [u8; CPIO_HEADER_SIZE]) {
        buf[0..6].copy_from_slice(CPIO_NEWC_MAGIC);
        write_hex_8(&mut buf[6..14], self.ino);
        write_hex_8(&mut buf[14..22], self.mode);
        write_hex_8(&mut buf[22..30], self.uid);
        write_hex_8(&mut buf[30..38], self.gid);
        write_hex_8(&mut buf[38..46], self.nlink);
        write_hex_8(&mut buf[46..54], self.mtime);
        write_hex_8(&mut buf[54..62], self.filesize);
        write_hex_8(&mut buf[62..70], self.devmajor);
        write_hex_8(&mut buf[70..78], self.devminor);
        write_hex_8(&mut buf[78..86], self.rdevmajor);
        write_hex_8(&mut buf[86..94], self.rdevminor);
        write_hex_8(&mut buf[94..102], self.namesize);
        write_hex_8(&mut buf[102..110], self.check);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpio_header_roundtrip() {
        let mut hdr = CpioHeader::default();
        hdr.ino = 100;
        hdr.mode = 0o100644;
        hdr.uid = 1000;
        hdr.gid = 1000;
        hdr.filesize = 42;
        hdr.namesize = 9;

        let mut buf = [0u8; CPIO_HEADER_SIZE];
        hdr.serialize(&mut buf);

        let parsed = CpioHeader::parse(&buf).expect("failed to parse");
        assert_eq!(parsed.ino, 100);
        assert_eq!(parsed.mode, 0o100644);
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.gid, 1000);
        assert_eq!(parsed.filesize, 42);
        assert_eq!(parsed.namesize, 9);
    }
}
