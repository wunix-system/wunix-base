// Формат архива ar (Unix Common Archive Format / System V).
// Используется для статических библиотек .a и пакетов .deb.

pub const AR_MAGIC: [u8; 8] = *b"!<arch>\n";
pub const AR_FMAG: [u8; 2] = [0x60, 0x0A]; // `\n
pub const HEADER_SIZE: usize = 60;

#[derive(Clone, Debug, Default)]
pub struct ArHeader {
    pub name: [u8; 16],
    pub mtime: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub size: u64,
}

fn parse_ascii_dec(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + ((b - b'0') as u64);
        } else if b != b' ' {
            break;
        }
    }
    val
}

fn parse_ascii_oct(bytes: &[u8]) -> u32 {
    let mut val = 0u32;
    for &b in bytes {
        if (b'0'..=b'7').contains(&b) {
            val = (val << 3) | ((b - b'0') as u32);
        } else if b != b' ' {
            break;
        }
    }
    val
}

fn write_ascii_dec(buf: &mut [u8], mut val: u64) {
    buf.fill(b' ');
    if val == 0 {
        buf[0] = b'0';
        return;
    }
    let mut temp = val;
    let mut digits = 0;
    while temp > 0 {
        digits += 1;
        temp /= 10;
    }
    let write_len = digits.min(buf.len());
    let mut cur = write_len;
    while val > 0 && cur > 0 {
        cur -= 1;
        buf[cur] = b'0' + (val % 10) as u8;
        val /= 10;
    }
}

fn write_ascii_oct(buf: &mut [u8], mut val: u32) {
    buf.fill(b' ');
    if val == 0 {
        buf[0] = b'0';
        return;
    }
    let mut temp = val;
    let mut digits = 0;
    while temp > 0 {
        digits += 1;
        temp >>= 3;
    }
    let write_len = digits.min(buf.len());
    let mut cur = write_len;
    while val > 0 && cur > 0 {
        cur -= 1;
        buf[cur] = b'0' + ((val & 7) as u8);
        val >>= 3;
    }
}

impl ArHeader {
    pub fn parse(buf: &[u8; HEADER_SIZE]) -> Option<Self> {
        if buf[58] != AR_FMAG[0] || buf[59] != AR_FMAG[1] {
            return None;
        }

        let mut name = [0u8; 16];
        name.copy_from_slice(&buf[0..16]);
        let mtime = parse_ascii_dec(&buf[16..28]);
        let uid = parse_ascii_dec(&buf[28..34]) as u32;
        let gid = parse_ascii_dec(&buf[34..40]) as u32;
        let mode = parse_ascii_oct(&buf[40..48]);
        let size = parse_ascii_dec(&buf[48..58]);

        Some(Self {
            name,
            mtime,
            uid,
            gid,
            mode,
            size,
        })
    }

    pub fn serialize(&self, buf: &mut [u8; HEADER_SIZE]) {
        buf.fill(b' ');
        buf[0..16].copy_from_slice(&self.name);
        write_ascii_dec(&mut buf[16..28], self.mtime);
        write_ascii_dec(&mut buf[28..34], self.uid as u64);
        write_ascii_dec(&mut buf[34..40], self.gid as u64);
        write_ascii_oct(&mut buf[40..48], self.mode);
        write_ascii_dec(&mut buf[48..58], self.size);
        buf[58..60].copy_from_slice(&AR_FMAG);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ar_header_roundtrip() {
        let mut hdr = ArHeader::default();
        let fname = b"libtest.o/";
        hdr.name[..fname.len()].copy_from_slice(fname);
        hdr.mtime = 1700000000;
        hdr.uid = 1000;
        hdr.gid = 1000;
        hdr.mode = 0o644;
        hdr.size = 1234;

        let mut buf = [0u8; HEADER_SIZE];
        hdr.serialize(&mut buf);

        let parsed = ArHeader::parse(&buf).expect("failed to parse");
        assert_eq!(&parsed.name[..fname.len()], fname);
        assert_eq!(parsed.mtime, 1700000000);
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.gid, 1000);
        assert_eq!(parsed.mode, 0o644);
        assert_eq!(parsed.size, 1234);
    }
}
