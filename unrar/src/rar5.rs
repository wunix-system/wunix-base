// Парсинг формата RAR 5.0+ (современный стандарт).
// Включает декодирование VINT (Variable-length integers) и разбор заголовков.

pub const RAR5_SIGNATURE: [u8; 8] = [0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x01, 0x00];

pub const HEAD_TYPE_MAIN: u64 = 1;
pub const HEAD_TYPE_FILE: u64 = 2;
pub const HEAD_TYPE_SERVICE: u64 = 3;
pub const HEAD_TYPE_CRYPT: u64 = 4;
pub const HEAD_TYPE_ENDARC: u64 = 5;

#[derive(Clone, Debug, Default)]
pub struct Rar5FileEntry {
    pub flags: u64,
    pub unp_size: u64,
    pub attr: u64,
    pub mtime: u32,
    pub data_size: u64,
    pub comp_info: u64,
    pub host_os: u64,
    pub name_len: usize,
    pub is_dir: bool,
    pub file_crc: u32,
    pub data_offset: u64,
}

// Декодирует переменной длины целое число (VINT)
pub fn read_vint(buf: &[u8], offset: &mut usize) -> Option<u64> {
    let mut val = 0u64;
    let mut shift = 0;
    while *offset < buf.len() && shift < 64 {
        let b = buf[*offset];
        *offset += 1;
        val |= ((b & 0x7F) as u64) << shift;
        if (b & 0x80) == 0 {
            return Some(val);
        }
        shift += 7;
    }
    None
}

// Парсит файловый заголовок RAR 5.x
pub fn parse_rar5_file_header(
    buf: &[u8],
    data_offset: u64,
) -> Option<(Rar5FileEntry, usize)> {
    let mut offset = 0usize;
    let file_flags = read_vint(buf, &mut offset)?;
    let unp_size = read_vint(buf, &mut offset)?;
    let attr = read_vint(buf, &mut offset)?;

    // Флаг наличия времени mtime
    let mut mtime = 0u32;
    if (file_flags & 0x02) != 0 {
        if offset + 4 > buf.len() {
            return None;
        }
        mtime = u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]]);
        offset += 4;
    }

    // Флаг наличия поля data_size (размер сжатых данных)
    let data_size = if (file_flags & 0x01) != 0 {
        read_vint(buf, &mut offset)?
    } else {
        0
    };

    let comp_info = read_vint(buf, &mut offset)?;
    let host_os = read_vint(buf, &mut offset)?;
    let name_len = read_vint(buf, &mut offset)? as usize;

    let is_dir = (file_flags & 0x01) == 0 && (file_flags & 0x04) != 0;

    Some((
        Rar5FileEntry {
            flags: file_flags,
            unp_size,
            attr,
            mtime,
            data_size,
            comp_info,
            host_os,
            name_len,
            is_dir,
            file_crc: 0,
            data_offset,
        },
        offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vint_decoding() {
        // Однобайтный VINT
        let buf1 = [0x05];
        let mut off1 = 0;
        assert_eq!(read_vint(&buf1, &mut off1), Some(5));
        assert_eq!(off1, 1);

        // Двухбайтный VINT (128 = 0x80 0x01)
        let buf2 = [0x80, 0x01];
        let mut off2 = 0;
        assert_eq!(read_vint(&buf2, &mut off2), Some(128));
        assert_eq!(off2, 2);
    }
}
