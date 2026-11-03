use libwunix::{close, open, read};

pub fn cstr_to_slice(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

pub fn read_file_str<'a>(path: &[u8], buf: &'a mut [u8]) -> &'a str {
    let fd = open(path, 0 /* O_RDONLY */, 0);
    if fd < 0 {
        return "";
    }
    let n = read(fd as u64, buf);
    let _ = close(fd as u64);
    if n > 0 {
        core::str::from_utf8(&buf[..(n as usize).min(buf.len())]).unwrap_or("")
    } else {
        ""
    }
}

pub fn parse_hex_byte(s: &[u8]) -> Option<u8> {
    if s.len() < 2 {
        return None;
    }
    let hi = match s[0] {
        b'0'..=b'9' => s[0] - b'0',
        b'a'..=b'f' => s[0] - b'a' + 10,
        b'A'..=b'F' => s[0] - b'A' + 10,
        _ => return None,
    };
    let lo = match s[1] {
        b'0'..=b'9' => s[1] - b'0',
        b'a'..=b'f' => s[1] - b'a' + 10,
        b'A'..=b'F' => s[1] - b'A' + 10,
        _ => return None,
    };
    Some((hi << 4) | lo)
}

pub fn parse_hex_ipv4_le(hex_s: &str) -> Option<[u8; 4]> {
    let bytes = hex_s.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    // В /proc/net/route адрес хранится в формате hex little-endian (младший байт первым)
    let b0 = parse_hex_byte(&bytes[0..2])?;
    let b1 = parse_hex_byte(&bytes[2..4])?;
    let b2 = parse_hex_byte(&bytes[4..6])?;
    let b3 = parse_hex_byte(&bytes[6..8])?;
    Some([b3, b2, b1, b0])
}
