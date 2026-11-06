// Парсинг архивов формата RAR 4.x (Eugene Roshal standard).
// Поддерживает маркерный блок, заголовок архива и файловые блоки.

pub const RAR4_SIGNATURE: [u8; 7] = [0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x00];

pub const HEAD_TYPE_MAIN: u8 = 0x73;
pub const HEAD_TYPE_FILE: u8 = 0x74;
pub const HEAD_TYPE_COMM: u8 = 0x75;
pub const HEAD_TYPE_ENDARC: u8 = 0x7b;

pub const FLAG_LONG_BLOCK: u16 = 0x8000;
pub const FLAG_LARGE_FILE: u16 = 0x0100;
pub const FLAG_DIRECTORY: u16 = 0x00e0; // В битах 5-7

pub const METHOD_STORE: u8 = 0x30;

#[derive(Clone, Debug, Default)]
pub struct Rar4FileEntry {
    pub pack_size: u64,
    pub unp_size: u64,
    pub host_os: u8,
    pub file_crc: u32,
    pub file_time: u32,
    pub unp_ver: u8,
    pub method: u8,
    pub name_len: usize,
    pub attr: u32,
    pub is_dir: bool,
    pub data_offset: u64,
}

// Парсит заголовок блока RAR 4.x
pub struct Rar4BlockHeader {
    pub head_crc: u16,
    pub head_type: u8,
    pub flags: u16,
    pub head_size: u16,
}

pub fn parse_block_header(buf: &[u8]) -> Option<Rar4BlockHeader> {
    if buf.len() < 7 {
        return None;
    }
    Some(Rar4BlockHeader {
        head_crc: u16::from_le_bytes([buf[0], buf[1]]),
        head_type: buf[2],
        flags: u16::from_le_bytes([buf[3], buf[4]]),
        head_size: u16::from_le_bytes([buf[5], buf[6]]),
    })
}

// Парсит тело заголовка файла RAR 4.x (HEAD_TYPE_FILE = 0x74)
pub fn parse_file_header(buf: &[u8], flags: u16, current_offset: u64) -> Option<(Rar4FileEntry, usize)> {
    if buf.len() < 25 {
        return None;
    }

    let mut pack_size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64;
    let mut unp_size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as u64;
    let host_os = buf[8];
    let file_crc = u32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]);
    let file_time = u32::from_le_bytes([buf[13], buf[14], buf[15], buf[16]]);
    let unp_ver = buf[17];
    let method = buf[18];
    let name_len = u16::from_le_bytes([buf[19], buf[20]]) as usize;
    let attr = u32::from_le_bytes([buf[21], buf[22], buf[23], buf[24]]);

    let mut offset = 25usize;
    if (flags & FLAG_LARGE_FILE) != 0 {
        if buf.len() < offset + 8 {
            return None;
        }
        let high_pack = u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]]) as u64;
        let high_unp = u32::from_le_bytes([buf[offset + 4], buf[offset + 5], buf[offset + 6], buf[offset + 7]]) as u64;
        pack_size |= high_pack << 32;
        unp_size |= high_unp << 32;
        offset += 8;
    }

    let is_dir = (flags & FLAG_DIRECTORY) == FLAG_DIRECTORY || (attr & 0x10) != 0;

    Some((
        Rar4FileEntry {
            pack_size,
            unp_size,
            host_os,
            file_crc,
            file_time,
            unp_ver,
            method,
            name_len,
            attr,
            is_dir,
            data_offset: current_offset,
        },
        offset,
    ))
}
