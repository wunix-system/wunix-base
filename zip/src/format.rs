// Структуры формата ZIP (PKWARE specification).
// Реализует сериализацию Local File Header, Central Directory Header и EOCD.

pub const LOCAL_FILE_HEADER_SIG: u32 = 0x04034b50;
pub const CENTRAL_DIR_HEADER_SIG: u32 = 0x02014b50;
pub const END_OF_CENTRAL_DIR_SIG: u32 = 0x06054b50;

pub const METHOD_STORE: u16 = 0;
pub const METHOD_DEFLATE: u16 = 8;

pub const LOCAL_HEADER_SIZE: usize = 30;
pub const CENTRAL_HEADER_SIZE: usize = 46;
pub const EOCD_SIZE: usize = 22;

#[derive(Clone, Copy, Debug)]
pub struct ZipEntryMeta {
    pub name_offset: usize,
    pub name_len: usize,
    pub method: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub local_header_offset: u32,
    pub mode: u32,
    pub is_dir: bool,
}

// Записывает 30-байтный Local File Header в буфер
pub fn write_local_header(
    buf: &mut [u8; LOCAL_HEADER_SIZE],
    method: u16,
    crc: u32,
    comp_size: u32,
    uncomp_size: u32,
    name_len: u16,
) {
    buf[0..4].copy_from_slice(&LOCAL_FILE_HEADER_SIG.to_le_bytes());
    buf[4..6].copy_from_slice(&20u16.to_le_bytes()); // Version needed (2.0)
    buf[6..8].copy_from_slice(&0u16.to_le_bytes());  // General purpose flags
    buf[8..10].copy_from_slice(&method.to_le_bytes());
    buf[10..12].copy_from_slice(&0u16.to_le_bytes()); // Mod time
    buf[12..14].copy_from_slice(&0u16.to_le_bytes()); // Mod date
    buf[14..18].copy_from_slice(&crc.to_le_bytes());
    buf[18..22].copy_from_slice(&comp_size.to_le_bytes());
    buf[22..26].copy_from_slice(&uncomp_size.to_le_bytes());
    buf[26..28].copy_from_slice(&name_len.to_le_bytes());
    buf[28..30].copy_from_slice(&0u16.to_le_bytes()); // Extra field length
}

// Записывает 46-байтный Central Directory Header в буфер
pub fn write_central_header(
    buf: &mut [u8; CENTRAL_HEADER_SIZE],
    entry: &ZipEntryMeta,
) {
    buf[0..4].copy_from_slice(&CENTRAL_DIR_HEADER_SIG.to_le_bytes());
    buf[4..6].copy_from_slice(&0x031eu16.to_le_bytes()); // Version made by (Unix, 3.0)
    buf[6..8].copy_from_slice(&20u16.to_le_bytes());     // Version needed (2.0)
    buf[8..10].copy_from_slice(&0u16.to_le_bytes());    // General purpose flags
    buf[10..12].copy_from_slice(&entry.method.to_le_bytes());
    buf[12..14].copy_from_slice(&0u16.to_le_bytes());   // Mod time
    buf[14..16].copy_from_slice(&0u16.to_le_bytes());   // Mod date
    buf[16..20].copy_from_slice(&entry.crc32.to_le_bytes());
    buf[20..24].copy_from_slice(&entry.compressed_size.to_le_bytes());
    buf[24..28].copy_from_slice(&entry.uncompressed_size.to_le_bytes());
    buf[28..30].copy_from_slice(&(entry.name_len as u16).to_le_bytes());
    buf[30..32].copy_from_slice(&0u16.to_le_bytes());   // Extra len
    buf[32..34].copy_from_slice(&0u16.to_le_bytes());   // Comment len
    buf[34..36].copy_from_slice(&0u16.to_le_bytes());   // Disk start
    buf[36..38].copy_from_slice(&0u16.to_le_bytes());   // Internal attributes
    // Внешние атрибуты: старшие 16 бит — POSIX права доступа, младшие — DOS биты
    let ext_attr = (entry.mode << 16) | if entry.is_dir { 0x10 } else { 0 };
    buf[38..42].copy_from_slice(&ext_attr.to_le_bytes());
    buf[42..46].copy_from_slice(&entry.local_header_offset.to_le_bytes());
}

// Записывает 22-байтный End of Central Directory Record (EOCD)
pub fn write_eocd(
    buf: &mut [u8; EOCD_SIZE],
    total_entries: u16,
    cd_size: u32,
    cd_offset: u32,
) {
    buf[0..4].copy_from_slice(&END_OF_CENTRAL_DIR_SIG.to_le_bytes());
    buf[4..6].copy_from_slice(&0u16.to_le_bytes()); // Disk num
    buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // Disk with start of CD
    buf[8..10].copy_from_slice(&total_entries.to_le_bytes());
    buf[10..12].copy_from_slice(&total_entries.to_le_bytes());
    buf[12..16].copy_from_slice(&cd_size.to_le_bytes());
    buf[16..20].copy_from_slice(&cd_offset.to_le_bytes());
    buf[20..22].copy_from_slice(&0u16.to_le_bytes()); // Comment len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eocd_serialization() {
        let mut buf = [0u8; EOCD_SIZE];
        write_eocd(&mut buf, 5, 230, 1024);
        assert_eq!(&buf[0..4], &END_OF_CENTRAL_DIR_SIG.to_le_bytes());
        assert_eq!(u16::from_le_bytes([buf[8], buf[9]]), 5);
        assert_eq!(u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]), 230);
        assert_eq!(u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]), 1024);
    }
}
