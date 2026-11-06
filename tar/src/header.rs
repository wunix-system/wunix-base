// Структуры и сериализация заголовка архива POSIX ustar (tar).
// Формат строго следует стандарту IEEE Std 1003.1-1988 (ustar).

pub const BLOCK_SIZE: usize = 512;
pub const USTAR_MAGIC: &[u8; 6] = b"ustar\0";
pub const USTAR_VERSION: &[u8; 2] = b"00";

pub const TYPE_REGULAR: u8 = b'0';
pub const TYPE_REGULAR_ALT: u8 = 0;
#[allow(dead_code)]
pub const TYPE_HARDLINK: u8 = b'1';
pub const TYPE_SYMLINK: u8 = b'2';
#[allow(dead_code)]
pub const TYPE_CHARDEV: u8 = b'3';
#[allow(dead_code)]
pub const TYPE_BLOCKDEV: u8 = b'4';
pub const TYPE_DIRECTORY: u8 = b'5';
#[allow(dead_code)]
pub const TYPE_FIFO: u8 = b'6';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TarEntryHeader {
    pub name: [u8; 100],
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub mtime: u64,
    pub typeflag: u8,
    pub linkname: [u8; 100],
    pub uname: [u8; 32],
    pub gname: [u8; 32],
    pub prefix: [u8; 155],
}

impl Default for TarEntryHeader {
    fn default() -> Self {
        Self {
            name: [0u8; 100],
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            typeflag: TYPE_REGULAR,
            linkname: [0u8; 100],
            uname: [0u8; 32],
            gname: [0u8; 32],
            prefix: [0u8; 155],
        }
    }
}

// Парсит восьмеричное число из текстового диапазона заголовка tar
pub fn parse_octal(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    let mut started = false;

    for &b in bytes {
        if (b'0'..=b'7').contains(&b) {
            val = (val << 3) | ((b - b'0') as u64);
            started = true;
        } else if b == 0 || b == b' ' {
            if started {
                break;
            }
        } else {
            break;
        }
    }
    val
}

// Записывает число в восьмеричном формате фиксированной длины с нуль-терминатором
pub fn write_octal(buf: &mut [u8], mut val: u64) {
    let len = buf.len();
    if len == 0 {
        return;
    }
    buf[len - 1] = 0; // Завершающий нуль
    let mut idx = len - 1;
    while idx > 0 {
        idx -= 1;
        buf[idx] = b'0' + ((val & 7) as u8);
        val >>= 3;
    }
}

// Извлекает строку до первого нуля
pub fn cstr_to_slice(buf: &[u8]) -> &[u8] {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..len]
}

impl TarEntryHeader {
    // Декодирует 512-байтный блок в структуру заголовка
    pub fn parse(block: &[u8; BLOCK_SIZE]) -> Option<Self> {
        // Проверка на блок конца архива (все нули)
        if block.iter().all(|&b| b == 0) {
            return None;
        }

        // Проверяем контрольную сумму: по стандарту сумма всех байт
        // блока, где поле chksum (148..156) заменено на 8 пробелов (0x20).
        let expected_chksum = parse_octal(&block[148..156]);
        let mut sum = 0u64;
        for (i, &b) in block.iter().enumerate() {
            if (148..156).contains(&i) {
                sum += b' ' as u64;
            } else {
                sum += b as u64;
            }
        }
        if sum != expected_chksum {
            return None;
        }

        let mut header = TarEntryHeader::default();
        header.name.copy_from_slice(&block[0..100]);
        header.mode = parse_octal(&block[100..108]) as u32;
        header.uid = parse_octal(&block[108..116]) as u32;
        header.gid = parse_octal(&block[116..124]) as u32;
        header.size = parse_octal(&block[124..136]);
        header.mtime = parse_octal(&block[136..148]);
        header.typeflag = block[156];
        header.linkname.copy_from_slice(&block[157..257]);
        header.uname.copy_from_slice(&block[265..297]);
        header.gname.copy_from_slice(&block[297..329]);
        header.prefix.copy_from_slice(&block[345..500]);

        Some(header)
    }

    // Сериализует заголовок в 512-байтный блок формата ustar
    pub fn serialize(&self, block: &mut [u8; BLOCK_SIZE]) {
        block.fill(0);

        block[0..100].copy_from_slice(&self.name);
        write_octal(&mut block[100..108], self.mode as u64);
        write_octal(&mut block[108..116], self.uid as u64);
        write_octal(&mut block[116..124], self.gid as u64);
        write_octal(&mut block[124..136], self.size);
        write_octal(&mut block[136..148], self.mtime);

        block[156] = self.typeflag;
        block[157..257].copy_from_slice(&self.linkname);

        block[257..263].copy_from_slice(USTAR_MAGIC);
        block[263..265].copy_from_slice(USTAR_VERSION);

        block[265..297].copy_from_slice(&self.uname);
        block[297..329].copy_from_slice(&self.gname);
        block[345..500].copy_from_slice(&self.prefix);

        // Вычисляем контрольную сумму (заполняем поле chksum пробелами)
        block[148..156].fill(b' ');
        let mut sum = 0u64;
        for &b in block.iter() {
            sum += b as u64;
        }

        // Записываем сумму в формате 6 цифр восьмеричного числа, null и пробел
        let mut chk_buf = [0u8; 8];
        write_octal(&mut chk_buf[0..7], sum);
        chk_buf[6] = 0;
        chk_buf[7] = b' ';
        block[148..156].copy_from_slice(&chk_buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octal_roundtrip() {
        let mut buf = [0u8; 8];
        write_octal(&mut buf, 0o755);
        assert_eq!(parse_octal(&buf), 0o755);
    }

    #[test]
    fn test_header_serialize_and_parse() {
        let mut hdr = TarEntryHeader::default();
        let fname = b"etc/motd";
        hdr.name[..fname.len()].copy_from_slice(fname);
        hdr.mode = 0o644;
        hdr.uid = 1000;
        hdr.gid = 1000;
        hdr.size = 42;
        hdr.mtime = 1700000000;
        hdr.typeflag = TYPE_REGULAR;

        let mut block = [0u8; BLOCK_SIZE];
        hdr.serialize(&mut block);

        let parsed = TarEntryHeader::parse(&block).expect("parse failed");
        assert_eq!(cstr_to_slice(&parsed.name), fname);
        assert_eq!(parsed.mode, 0o644);
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.gid, 1000);
        assert_eq!(parsed.size, 42);
        assert_eq!(parsed.typeflag, TYPE_REGULAR);
    }
}
