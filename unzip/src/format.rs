// Парсинг структур ZIP архива (EOCD, Central Directory, Local Header).

use libwunix::{lseek, read, SEEK_SET};

pub const LOCAL_FILE_HEADER_SIG: u32 = 0x04034b50;
pub const CENTRAL_DIR_HEADER_SIG: u32 = 0x02014b50;
pub const END_OF_CENTRAL_DIR_SIG: u32 = 0x06054b50;

pub const METHOD_STORE: u16 = 0;
pub const METHOD_DEFLATE: u16 = 8;

pub const LOCAL_HEADER_SIZE: usize = 30;
pub const EOCD_MIN_SIZE: usize = 22;
pub const CENTRAL_HEADER_SIZE: usize = 46;

#[derive(Clone, Copy, Debug)]
pub struct EocdInfo {
    pub total_entries: u16,
    pub cd_size: u32,
    pub cd_offset: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CentralDirEntry {
    pub method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub name_len: u16,
    pub extra_len: u16,
    pub comment_len: u16,
    pub external_attr: u32,
    pub local_header_offset: u32,
}

impl CentralDirEntry {
    pub fn is_dir(&self, name: &[u8]) -> bool {
        (self.external_attr & 0x10) != 0 || name.ends_with(b"/")
    }

    pub fn mode(&self) -> u32 {
        let posix_mode = (self.external_attr >> 16) & 0o7777;
        if posix_mode != 0 {
            posix_mode
        } else if (self.external_attr & 0x10) != 0 {
            0o755
        } else {
            0o644
        }
    }
}

// Находит запись EOCD, сканируя конец файла
pub fn find_eocd(fd: u64, file_size: u64) -> Result<EocdInfo, ()> {
    if file_size < EOCD_MIN_SIZE as u64 {
        return Err(());
    }

    let scan_len = (file_size).min(65536 + EOCD_MIN_SIZE as u64) as usize;
    let seek_pos = file_size - scan_len as u64;
    lseek(fd, seek_pos as i64, SEEK_SET as i32);

    let mut buf = [0u8; 4096];
    let mut window = [0u8; 4096];
    let mut window_len = 0;

    let mut remaining = scan_len;
    let mut found_eocd: Option<EocdInfo> = None;

    while remaining > 0 {
        let to_read = remaining.min(buf.len());
        let rd = read(fd, &mut buf[..to_read]);
        if rd == 0 {
            break;
        }
        let chunk = rd as usize;

        let combined_len = window_len + chunk;
        let mut temp = [0u8; 8192];
        temp[..window_len].copy_from_slice(&window[..window_len]);
        temp[window_len..combined_len].copy_from_slice(&buf[..chunk]);

        if combined_len >= EOCD_MIN_SIZE {
            let mut i = 0;
            while i <= combined_len - EOCD_MIN_SIZE {
                let sig = u32::from_le_bytes([temp[i], temp[i + 1], temp[i + 2], temp[i + 3]]);
                if sig == END_OF_CENTRAL_DIR_SIG {
                    let total_entries = u16::from_le_bytes([temp[i + 10], temp[i + 11]]);
                    let cd_size = u32::from_le_bytes([temp[i + 12], temp[i + 13], temp[i + 14], temp[i + 15]]);
                    let cd_offset = u32::from_le_bytes([temp[i + 16], temp[i + 17], temp[i + 18], temp[i + 19]]);
                    found_eocd = Some(EocdInfo {
                        total_entries,
                        cd_size,
                        cd_offset,
                    });
                }
                i += 1;
            }
        }

        window_len = chunk.min(4096);
        window[..window_len].copy_from_slice(&buf[chunk - window_len..chunk]);
        remaining -= chunk;
    }

    found_eocd.ok_or(())
}

// Парсит одну запись Центрального Каталога из байтового среза
pub fn parse_central_dir_entry(data: &[u8]) -> Option<CentralDirEntry> {
    if data.len() < CENTRAL_HEADER_SIZE {
        return None;
    }
    let sig = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if sig != CENTRAL_DIR_HEADER_SIG {
        return None;
    }

    Some(CentralDirEntry {
        method: u16::from_le_bytes([data[10], data[11]]),
        mod_time: u16::from_le_bytes([data[12], data[13]]),
        mod_date: u16::from_le_bytes([data[14], data[15]]),
        crc32: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
        compressed_size: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
        uncompressed_size: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
        name_len: u16::from_le_bytes([data[28], data[29]]),
        extra_len: u16::from_le_bytes([data[30], data[31]]),
        comment_len: u16::from_le_bytes([data[32], data[33]]),
        external_attr: u32::from_le_bytes([data[38], data[39], data[40], data[41]]),
        local_header_offset: u32::from_le_bytes([data[42], data[43], data[44], data[45]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_central_dir_entry() {
        let mut raw = [0u8; CENTRAL_HEADER_SIZE];
        raw[0..4].copy_from_slice(&CENTRAL_DIR_HEADER_SIG.to_le_bytes());
        raw[10..12].copy_from_slice(&8u16.to_le_bytes()); // Deflate
        raw[20..24].copy_from_slice(&100u32.to_le_bytes()); // comp
        raw[24..28].copy_from_slice(&250u32.to_le_bytes()); // uncomp
        raw[28..30].copy_from_slice(&8u16.to_le_bytes()); // name_len

        let entry = parse_central_dir_entry(&raw).expect("failed to parse");
        assert_eq!(entry.method, 8);
        assert_eq!(entry.compressed_size, 100);
        assert_eq!(entry.uncompressed_size, 250);
        assert_eq!(entry.name_len, 8);
    }
}

