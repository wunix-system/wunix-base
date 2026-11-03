use libwunix::{close, open, read};

#[derive(Clone, Copy)]
pub struct PciDevEntry<'a> {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub irq: u8,
    pub bars: [u64; 6],
    pub bar_sizes: [u64; 6],
    pub driver: &'a str,
}

pub fn parse_hex(s: &str) -> Option<u64> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    if s.is_empty() {
        return None;
    }
    let mut val = 0u64;
    for b in s.bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u64,
            b'a'..=b'f' => (b - b'a' + 10) as u64,
            b'A'..=b'F' => (b - b'A' + 10) as u64,
            _ => return None,
        };
        val = val.checked_mul(16)?.checked_add(digit)?;
    }
    Some(val)
}

pub fn load_pci_devices<'a>(
    content: &'a str,
    entries: &mut [Option<PciDevEntry<'a>>; 32],
) -> usize {
    let mut count = 0usize;
    for line in content.lines() {
        if line.is_empty() || count >= 32 {
            continue;
        }
        let mut parts = line.split_whitespace();
        let dfn_str = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        let vend_str = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        let irq_str = match parts.next() {
            Some(s) => s,
            None => continue,
        };

        let dfn = parse_hex(dfn_str).unwrap_or(0);
        let bus = (dfn >> 8) as u8;
        let dev = ((dfn >> 3) & 0x1F) as u8;
        let func = (dfn & 0x07) as u8;

        let vend_val = parse_hex(vend_str).unwrap_or(0);
        let vendor_id = (vend_val >> 16) as u16;
        let device_id = (vend_val & 0xFFFF) as u16;
        let irq = parse_hex(irq_str).unwrap_or(0) as u8;

        let mut bars = [0u64; 6];
        for b in &mut bars {
            if let Some(bs) = parts.next() {
                *b = parse_hex(bs).unwrap_or(0);
            }
        }

        let mut bar_sizes = [0u64; 6];
        for sz in &mut bar_sizes {
            if let Some(ss) = parts.next() {
                *sz = parse_hex(ss).unwrap_or(0);
            }
        }

        let driver = parts.next().unwrap_or("-");
        let class_hex_str = parts.next();
        let (class_code, subclass) = if let Some(chs) = class_hex_str {
            let val = parse_hex(chs).unwrap_or(0) as u16;
            ((val >> 8) as u8, (val & 0xFF) as u8)
        } else {
            (0, 0)
        };

        entries[count] = Some(PciDevEntry {
            bus,
            dev,
            func,
            vendor_id,
            device_id,
            class_code,
            subclass,
            irq,
            bars,
            bar_sizes,
            driver,
        });
        count += 1;
    }
    count
}

pub fn read_proc_pci_devices(buf: &mut [u8]) -> usize {
    let mut file_len = 0usize;
    let fd = open(b"/proc/bus/pci/devices\0", 0, 0);
    if fd >= 0 {
        let n = read(fd as u64, buf);
        if n > 0 {
            file_len = n as usize;
        }
        close(fd as u64);
    }
    file_len
}
