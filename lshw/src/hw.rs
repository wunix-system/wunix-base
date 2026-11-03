use libwunix::{close, open, read, sysinfo};

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct SysInfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
    pub _f: [u8; 20 - 2 * core::mem::size_of::<u64>() - core::mem::size_of::<u32>()],
}

#[derive(Clone, Copy)]
pub struct DiskInfo {
    pub name: [u8; 16],
    pub name_len: usize,
    pub size_mb: u64,
}

pub struct HwInventory<'a> {
    pub total_ram_mb: u64,
    pub free_ram_mb: u64,
    pub cpu_vendor: &'a str,
    pub cpu_brand: &'a str,
    pub fb_mode: &'a str,
    pub net_mac: &'a str,
    pub disks: [DiskInfo; 8],
    pub disk_count: usize,
}

pub fn read_file<'a>(path: &[u8], buf: &'a mut [u8]) -> &'a str {
    let fd = open(path, 0, 0);
    if fd < 0 {
        return "";
    }
    let n = read(fd as u64, buf);
    close(fd as u64);
    if n > 0 {
        core::str::from_utf8(&buf[..(n as usize).min(buf.len())]).unwrap_or("")
    } else {
        ""
    }
}

pub fn parse_u64(s: &str) -> u64 {
    let mut val = 0u64;
    for b in s.bytes() {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u64;
        } else {
            break;
        }
    }
    val
}

pub struct HwBuffers {
    pub cpu_buf: [u8; 1024],
    pub fb_buf: [u8; 128],
    pub mac_buf: [u8; 64],
    pub part_buf: [u8; 2048],
}

impl HwBuffers {
    pub const fn new() -> Self {
        Self {
            cpu_buf: [0; 1024],
            fb_buf: [0; 128],
            mac_buf: [0; 64],
            part_buf: [0; 2048],
        }
    }
}

pub fn collect_inventory<'a>(bufs: &'a mut HwBuffers) -> HwInventory<'a> {
    let mut sinfo = SysInfo::default();
    let _ = sysinfo(&mut sinfo as *mut _ as *mut u8);
    let mem_unit = if sinfo.mem_unit > 0 { sinfo.mem_unit as u64 } else { 1 };
    let total_ram_mb = (sinfo.totalram * mem_unit) / (1024 * 1024);
    let free_ram_mb = (sinfo.freeram * mem_unit) / (1024 * 1024);

    let cpu_text = read_file(b"/proc/cpuinfo\0", &mut bufs.cpu_buf);
    let mut cpu_vendor = "x86_64";
    let mut cpu_brand = "x86_64 Processor";
    for line in cpu_text.lines() {
        if line.starts_with("vendor_id") {
            if let Some(pos) = line.find(':') {
                cpu_vendor = line[pos + 1..].trim();
            }
        } else if line.starts_with("model name") {
            if let Some(pos) = line.find(':') {
                cpu_brand = line[pos + 1..].trim();
            }
        }
    }

    let fb_text = read_file(b"/sys/class/graphics/fb0/modes\0", &mut bufs.fb_buf);
    let fb_mode = fb_text.lines().next().unwrap_or("1024x768-32").trim();

    let mac_text = read_file(b"/sys/class/net/eth0/address\0", &mut bufs.mac_buf);
    let net_mac = if !mac_text.trim().is_empty() { mac_text.trim() } else { "N/A" };

    let part_text = read_file(b"/proc/partitions\0", &mut bufs.part_buf);
    let mut disks = [DiskInfo { name: [0; 16], name_len: 0, size_mb: 0 }; 8];
    let mut disk_count = 0usize;

    for (line_idx, line) in part_text.lines().enumerate() {
        if line_idx == 0 || line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let _maj = match parts.next() { Some(m) => m, None => continue };
        let _min = match parts.next() { Some(m) => m, None => continue };
        let blocks_str = match parts.next() { Some(b) => b, None => continue };
        let name = match parts.next() { Some(n) => n, None => continue };

        let blocks = parse_u64(blocks_str);
        let size_mb = blocks / 1024;
        let nb = name.as_bytes();

        if disk_count < disks.len() {
            let slot = &mut disks[disk_count];
            let len = nb.len().min(slot.name.len());
            slot.name[..len].copy_from_slice(&nb[..len]);
            slot.name_len = len;
            slot.size_mb = size_mb;
            disk_count += 1;
        }
    }

    if disk_count == 0 {
        let slot = &mut disks[0];
        let def = b"vda";
        slot.name[..def.len()].copy_from_slice(def);
        slot.name_len = def.len();
        slot.size_mb = 64;
        disk_count = 1;
    }

    HwInventory {
        total_ram_mb,
        free_ram_mb,
        cpu_vendor,
        cpu_brand,
        fb_mode,
        net_mac,
        disks,
        disk_count,
    }
}
