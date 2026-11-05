// Обнаружение дисков, планирование разделов, запись MBR и форматирование FAT32/Swap.

use libwunix::{close, lseek, open, pread64, pwrite64, O_RDONLY, O_RDWR, SEEK_END};

pub const MAX_DISKS: usize = 8;
pub const MAX_PARTITIONS: usize = 4;

const CANDIDATES: &[&[u8]] = &[
    b"/dev/vda\0",
    b"/dev/sda\0",
    b"/dev/nvme0n1\0",
    b"/dev/hda\0",
    b"/dev/vdb\0",
    b"/dev/sdb\0",
];

#[derive(Copy, Clone)]
pub struct DiskInfo {
    pub path: &'static [u8], // null-terminated
    pub sector_count: u64,
}

impl DiskInfo {
    pub fn name(&self) -> &str {
        let end = self.path.iter().position(|&b| b == 0).unwrap_or(self.path.len());
        core::str::from_utf8(&self.path[..end]).unwrap_or("?")
    }

    pub fn size_mib(&self) -> u64 {
        self.sector_count / 2048
    }

    #[allow(dead_code)]
    pub fn size_gib(&self) -> u64 {
        self.size_mib() / 1024
    }
}

impl Default for DiskInfo {
    fn default() -> Self {
        Self { path: b"\0", sector_count: 0 }
    }
}

/// Точное определение объема диска через системный вызов lseek(SEEK_END)
fn probe(path: &'static [u8]) -> Option<DiskInfo> {
    let fd = open(path, O_RDONLY as u64, 0);
    if fd < 0 { return None; }

    let total_bytes = lseek(fd as u64, 0, SEEK_END);
    close(fd as u64);

    if total_bytes <= 0 {
        // Запасной опрос: проверяем доступность первого сектора
        let fd2 = open(path, O_RDONLY as u64, 0);
        if fd2 < 0 { return None; }
        let mut buf = [0u8; 512];
        let n = pread64(fd2 as u64, &mut buf, 0);
        close(fd2 as u64);
        if n <= 0 { return None; }
        return Some(DiskInfo { path, sector_count: 1048576 }); // 512 MiB fallback
    }

    let sectors = (total_bytes as u64) / 512;
    Some(DiskInfo { path, sector_count: sectors })
}

pub fn scan_disks(out: &mut [DiskInfo; MAX_DISKS]) -> usize {
    let mut count = 0;
    for &c in CANDIDATES {
        if count >= MAX_DISKS { break; }
        if let Some(info) = probe(c) {
            out[count] = info;
            count += 1;
        }
    }
    count
}

// ─── Структуры разметки разделов ──────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct PartitionPlan {
    #[allow(dead_code)]
    pub num: u8,
    pub mount_point: &'static str,
    pub fstype: &'static str,
    pub start_lba: u32,
    pub sector_count: u32,
    pub bootable: bool,
}

impl PartitionPlan {
    pub const fn empty() -> Self {
        Self {
            num: 0,
            mount_point: "",
            fstype: "",
            start_lba: 0,
            sector_count: 0,
            bootable: false,
        }
    }

    pub fn size_mib(&self) -> u64 {
        (self.sector_count as u64) / 2048
    }
}

/// Создание автоматической рекомендованной схемы разметки
pub fn build_auto_partition_plan(
    total_sectors: u64,
    selected_fs: &'static str,
    out: &mut [PartitionPlan; MAX_PARTITIONS],
) -> usize {
    for p in out.iter_mut() {
        *p = PartitionPlan::empty();
    }

    // Если диск более 512 MiB:
    // Part 1: EFI/Boot (FAT32, 64 MiB = 131072 сектора, bootable)
    // Part 2: Swap (Linux swap, 256 MiB = 524288 секторов)
    // Part 3: Root / (выбранная ФС, остальное пространство)
    if total_sectors >= 1_048_576 {
        let boot_sectors: u32 = 131072; // 64 MiB
        let swap_sectors: u32 = 524288; // 256 MiB
        let start_root = 2048 + boot_sectors + swap_sectors;
        let root_sectors = (total_sectors.saturating_sub(start_root as u64)).min(0xFFFF_FFFF) as u32;

        out[0] = PartitionPlan {
            num: 1,
            mount_point: "/boot",
            fstype: "fat32",
            start_lba: 2048,
            sector_count: boot_sectors,
            bootable: true,
        };
        out[1] = PartitionPlan {
            num: 2,
            mount_point: "swap",
            fstype: "swap",
            start_lba: 2048 + boot_sectors,
            sector_count: swap_sectors,
            bootable: false,
        };
        out[2] = PartitionPlan {
            num: 3,
            mount_point: "/",
            fstype: selected_fs,
            start_lba: start_root,
            sector_count: root_sectors,
            bootable: false,
        };
        3
    } else if total_sectors >= 400_000 {
        // Диск от 200 до 512 MiB:
        // Part 1: Root / (FAT32, bootable)
        // Part 2: Swap (64 MiB)
        let swap_sectors: u32 = 131072; // 64 MiB
        let root_sectors = (total_sectors.saturating_sub(2048 + swap_sectors as u64)).min(0xFFFF_FFFF) as u32;

        out[0] = PartitionPlan {
            num: 1,
            mount_point: "/",
            fstype: selected_fs,
            start_lba: 2048,
            sector_count: root_sectors,
            bootable: true,
        };
        out[1] = PartitionPlan {
            num: 2,
            mount_point: "swap",
            fstype: "swap",
            start_lba: 2048 + root_sectors,
            sector_count: swap_sectors,
            bootable: false,
        };
        2
    } else {
        // Небольшой диск: единый раздел под корень
        let root_sectors = (total_sectors.saturating_sub(2048)).min(0xFFFF_FFFF) as u32;
        out[0] = PartitionPlan {
            num: 1,
            mount_point: "/",
            fstype: selected_fs,
            start_lba: 2048,
            sector_count: root_sectors,
            bootable: true,
        };
        1
    }
}

// ─── Валидация разделов ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
pub struct PartitionValidation {
    pub has_root: bool,
    pub has_swap: bool,
    pub has_boot: bool,
    pub root_size_ok: bool,
    pub is_valid: bool,
}

pub fn validate_partition_plan(plans: &[PartitionPlan], count: usize) -> PartitionValidation {
    let mut v = PartitionValidation::default();
    for i in 0..count {
        let p = &plans[i];
        if p.mount_point == "/" {
            v.has_root = true;
            if p.sector_count >= 100_000 {
                v.root_size_ok = true;
            }
        }
        if p.mount_point == "swap" || p.fstype == "swap" {
            v.has_swap = true;
        }
        if p.mount_point == "/boot" || p.bootable {
            v.has_boot = true;
        }
    }
    v.is_valid = v.has_root;
    v
}

// ─── Запись MBR и форматирование ─────────────────────────────────────────────

/// Зачищает первые 64 сектора диска для удаления старых таблиц MBR/GPT/сигнатур
pub fn wipe_disk_signatures(disk_path: &[u8]) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    let zero = [0u8; 512];
    for sec in 0..64 {
        let _ = pwrite64(fd as u64, &zero, sec * 512);
    }
    close(fd as u64);
    true
}

/// Записывает MBR таблицу разделов в сектор 0 целевого диска
pub fn write_mbr_table(disk_path: &[u8], plans: &[PartitionPlan], count: usize) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    let mut mbr = [0u8; 512];

    for i in 0..count.min(4) {
        let p = &plans[i];
        let offset = 446 + i * 16;
        let pe = &mut mbr[offset..offset + 16];

        pe[0] = if p.bootable { 0x80 } else { 0x00 };
        pe[1] = 0x00; pe[2] = 0x01; pe[3] = 0x00; // CHS start

        pe[4] = if p.fstype == "swap" || p.mount_point == "swap" {
            0x82 // Linux swap
        } else if p.mount_point == "/boot" {
            0x0C // FAT32 LBA
        } else if p.fstype == "ext4" {
            0x83 // Linux native
        } else {
            0x0C // FAT32 LBA
        };

        pe[5] = 0xFE; pe[6] = 0xFF; pe[7] = 0xFF; // CHS end
        pe[8..12].copy_from_slice(&p.start_lba.to_le_bytes());
        pe[12..16].copy_from_slice(&p.sector_count.to_le_bytes());
    }

    mbr[510] = 0x55;
    mbr[511] = 0xAA;

    let ok = pwrite64(fd as u64, &mbr, 0) == 512;
    close(fd as u64);
    ok
}

/// Инициализация заголовка swap-раздела
pub fn init_swap(disk_path: &[u8], lba_offset: u64, _part_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    // Swap signature "SWAPSPACE2" записывается в конец 1-й страницы (байт 4086..4096)
    let sig_offset = lba_offset * 512 + 4086;
    let ok = pwrite64(fd as u64, b"SWAPSPACE2", sig_offset) == 10;
    close(fd as u64);
    ok
}

/// Форматирует раздел (lba_offset в секторах) как валидную FAT32.
pub fn format_fat32(disk_path: &[u8], lba_offset: u64, part_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    let byte_off = lba_offset * 512;
    let sec_per_clus: u8 = if part_sectors >= 530_000 { 8 } else if part_sectors >= 140_000 { 2 } else { 1 };
    let rsvd_sec: u16 = 32;
    let num_fats: u8 = 2;

    let est_clusters = (part_sectors.saturating_sub(rsvd_sec as u64)) / (sec_per_clus as u64);
    let fat_sectors = (((est_clusters + 2) * 4 + 511) / 512) as u32;

    let mut bpb = [0u8; 512];
    bpb[0] = 0xEB; bpb[1] = 0x58; bpb[2] = 0x90;
    bpb[3..11].copy_from_slice(b"WUNIX   ");
    bpb[11] = 0x00; bpb[12] = 0x02;                  // 512 байт на сектор
    bpb[13] = sec_per_clus;                          // секторов на кластер
    bpb[14] = (rsvd_sec & 0xFF) as u8;
    bpb[15] = ((rsvd_sec >> 8) & 0xFF) as u8;        // зарезервированные сектора
    bpb[16] = num_fats;                              // 2 таблицы FAT
    bpb[17] = 0;   bpb[18] = 0;                      // root entries = 0 (FAT32)
    bpb[19] = 0;   bpb[20] = 0;                      // total sectors 16 = 0
    bpb[21] = 0xF8;                                   // media: fixed disk
    bpb[22] = 0;   bpb[23] = 0;                      // sectors/FAT 16 = 0
    bpb[24] = 63;  bpb[25] = 0;                      // sectors/track
    bpb[26] = 255; bpb[27] = 0;                      // heads
    bpb[28..32].copy_from_slice(&(lba_offset as u32).to_le_bytes()); // hidden sectors
    bpb[32..36].copy_from_slice(&(part_sectors as u32).to_le_bytes()); // total sectors 32

    // Расширенный BPB FAT32
    bpb[36..40].copy_from_slice(&fat_sectors.to_le_bytes());
    bpb[40] = 0; bpb[41] = 0;                         // ext flags
    bpb[42] = 0; bpb[43] = 0;                         // FS version 0.0
    bpb[44] = 2; bpb[45] = 0; bpb[46] = 0; bpb[47] = 0; // корневой кластер 2
    bpb[48] = 1; bpb[49] = 0;                         // FSInfo сектор 1
    bpb[50] = 6; bpb[51] = 0;                         // Backup boot сектор 6
    bpb[64] = 0x80;                                   // drive number
    bpb[66] = 0x29;                                   // boot signature
    bpb[67] = 0xEF; bpb[68] = 0xBE; bpb[69] = 0xAD; bpb[70] = 0xDE; // volume ID
    bpb[71..82].copy_from_slice(b"WUNIX      ");
    bpb[82..90].copy_from_slice(b"FAT32   ");
    bpb[510] = 0x55; bpb[511] = 0xAA;

    let mut ok = pwrite64(fd as u64, &bpb, byte_off) == 512;
    let _ = pwrite64(fd as u64, &bpb, byte_off + 6 * 512);

    // Сектор 1: FSInfo
    let mut fsinfo = [0u8; 512];
    fsinfo[0..4].copy_from_slice(b"RRaA");
    fsinfo[484..488].copy_from_slice(b"rrAa");
    fsinfo[488..492].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    fsinfo[492..496].copy_from_slice(&2u32.to_le_bytes());
    fsinfo[510] = 0x55; fsinfo[511] = 0xAA;
    let _ = pwrite64(fd as u64, &fsinfo, byte_off + 512);

    // Инициализация таблиц FAT
    let fat1_off = byte_off + (rsvd_sec as u64) * 512;
    let fat2_off = fat1_off + fat_sectors as u64 * 512;
    let mut fat_hdr = [0u8; 512];
    fat_hdr[0] = 0xF8; fat_hdr[1] = 0xFF; fat_hdr[2] = 0xFF; fat_hdr[3] = 0x0F;
    fat_hdr[4] = 0xFF; fat_hdr[5] = 0xFF; fat_hdr[6] = 0xFF; fat_hdr[7] = 0x0F;
    fat_hdr[8] = 0xFF; fat_hdr[9] = 0xFF; fat_hdr[10] = 0xFF; fat_hdr[11] = 0x0F;
    ok &= pwrite64(fd as u64, &fat_hdr, fat1_off) == 512;
    ok &= pwrite64(fd as u64, &fat_hdr, fat2_off) == 512;

    // Обнуляем корневой кластер
    let data_start = fat2_off + fat_sectors as u64 * 512;
    let zero = [0u8; 512];
    for c_sec in 0..(sec_per_clus as u64) {
        let _ = pwrite64(fd as u64, &zero, data_start + c_sec * 512);
    }

    close(fd as u64);
    ok
}

/// Устанавливает загрузочную запись Limine (Stage 1 и Stage 2) на целевой диск
pub fn install_limine_bootloader(disk_path: &[u8]) -> bool {
    let fd_bin = open(b"/boot/limine_hdd.bin\0", O_RDONLY as u64, 0);
    if fd_bin < 0 {
        return true;
    }

    let fd_disk = open(disk_path, O_RDWR as u64, 0);
    if fd_disk < 0 {
        close(fd_bin as u64);
        return false;
    }

    let mut boot_sector = [0u8; 512];
    let n = pread64(fd_bin as u64, &mut boot_sector, 0);
    if n != 512 {
        close(fd_bin as u64);
        close(fd_disk as u64);
        return false;
    }

    let mut orig_part_table = [0u8; 70];
    let _ = pread64(fd_disk as u64, &mut orig_part_table, 440);

    let stage2_size_a: u16 = 10752;
    let stage2_size_b: u16 = 10752;
    let stage2_loc_a: u64 = 0x200;
    let stage2_loc_b: u64 = 0x2c00;

    boot_sector[420..422].copy_from_slice(&stage2_size_a.to_le_bytes());
    boot_sector[422..424].copy_from_slice(&stage2_size_b.to_le_bytes());
    boot_sector[424..432].copy_from_slice(&stage2_loc_a.to_le_bytes());
    boot_sector[432..440].copy_from_slice(&stage2_loc_b.to_le_bytes());

    boot_sector[440..510].copy_from_slice(&orig_part_table);
    boot_sector[510] = 0x55;
    boot_sector[511] = 0xAA;

    let _ = pwrite64(fd_disk as u64, &boot_sector, 0);

    let mut buf = [0u8; 4096];
    let mut off = 512u64;
    loop {
        let r = pread64(fd_bin as u64, &mut buf, off);
        if r <= 0 { break; }
        let _ = pwrite64(fd_disk as u64, &buf[..r as usize], off);
        off += r as u64;
    }

    close(fd_bin as u64);
    close(fd_disk as u64);
    true
}
