// Обнаружение дисков, планирование разделов, запись MBR и форматирование ФС.

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
        } else if p.mount_point == "/boot" || p.fstype == "fat32" || p.fstype == "fat" {
            0x0C // FAT32 LBA — загрузочный раздел Limine
        } else if p.fstype == "ext4" {
            0x83 // Linux native (ext2/3/4)
        } else if p.fstype == "ufs" || p.fstype == "ufs2" {
            0xA5 // BSD/UFS — стандартный тип раздела для UFS
        } else {
            0xA5 // По умолчанию UFS для разделов Wunix
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

/// Форматирует раздел как журналируемую файловую систему Ext4 (JBD2).
///
/// Записывает стандартную структуру Ext4:
/// - Суперблок (0xEF53) с поддержкой EXTENTS, FILETYPE и журнала JBD2
/// - Дескрипторы групп блоков и битовые карты аллокации
/// - Таблицу инодов с корневым каталогом (inode 2) и инодом журнала (inode 8)
/// - Начальный блок корневого каталога с записями '.' и '..'
/// - Суперблок журнала JBD2 (0xC036390C)
pub fn format_ext4(disk_path: &[u8], lba_offset: u64, part_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 {
        return false;
    }

    let block_size = 4096u64;
    let total_blocks = (part_sectors / 8).max(1024) as u32;
    let inodes_count = 2048u32;
    let blocks_per_group = 32768u32.min(total_blocks);
    let inodes_per_group = inodes_count;

    // 1. Суперблок Ext4 (LBA lba_offset + 2, байтовое смещение 1024 от начала раздела)
    let mut sb = [0u8; 1024];
    sb[0..4].copy_from_slice(&inodes_count.to_le_bytes());
    sb[4..8].copy_from_slice(&total_blocks.to_le_bytes());
    sb[12..16].copy_from_slice(&(total_blocks.saturating_sub(1150)).to_le_bytes()); // free blocks
    sb[16..20].copy_from_slice(&(inodes_count - 11).to_le_bytes());                 // free inodes
    sb[20..24].copy_from_slice(&0u32.to_le_bytes());                                // first data block (0 для 4K)
    sb[24..28].copy_from_slice(&2u32.to_le_bytes());                                // log_block_size = 2 (4096)
    sb[32..36].copy_from_slice(&blocks_per_group.to_le_bytes());
    sb[40..44].copy_from_slice(&inodes_per_group.to_le_bytes());
    sb[56..58].copy_from_slice(&0xEF53u16.to_le_bytes());                           // s_magic
    sb[58..60].copy_from_slice(&1u16.to_le_bytes());                                // s_state = clean
    sb[84..88].copy_from_slice(&11u32.to_le_bytes());                               // s_first_ino = 11
    sb[88..90].copy_from_slice(&256u16.to_le_bytes());                              // s_inode_size = 256
    // s_feature_compat: HAS_JOURNAL (0x04) | DIR_INDEX (0x20)
    sb[92..96].copy_from_slice(&0x0024u32.to_le_bytes());
    // s_feature_incompat: FILETYPE (0x02) | EXTENTS (0x40)
    sb[96..100].copy_from_slice(&0x0042u32.to_le_bytes());
    // s_feature_ro_compat: SPARSE_SUPER (0x01) | LARGE_FILE (0x02)
    sb[100..104].copy_from_slice(&0x0003u32.to_le_bytes());
    sb[224..228].copy_from_slice(&8u32.to_le_bytes());                              // s_journal_inum = 8

    let sb_byte_offset = lba_offset * 512 + 1024;
    if pwrite64(fd as u64, &sb, sb_byte_offset) != 1024 {
        close(fd as u64);
        return false;
    }

    // 2. Дескриптор группы блоков (блок 1, смещение 4096)
    let mut bg_desc = [0u8; 32];
    bg_desc[0..4].copy_from_slice(&2u32.to_le_bytes());  // block_bitmap = block 2
    bg_desc[4..8].copy_from_slice(&3u32.to_le_bytes());  // inode_bitmap = block 3
    bg_desc[8..12].copy_from_slice(&4u32.to_le_bytes()); // inode_table = block 4
    bg_desc[12..14].copy_from_slice(&(total_blocks.saturating_sub(1150) as u16).to_le_bytes());
    bg_desc[14..16].copy_from_slice(&((inodes_count - 11) as u16).to_le_bytes());
    bg_desc[16..18].copy_from_slice(&2u16.to_le_bytes()); // used_dirs = 2

    let mut bg_buf = [0u8; 4096];
    bg_buf[0..32].copy_from_slice(&bg_desc);
    let bg_offset = lba_offset * 512 + block_size;
    let _ = pwrite64(fd as u64, &bg_buf, bg_offset);

    // 3. Битовая карта блоков (блок 2)
    // Занято 1150 блоков: 0..131 (метаданные), 132 (root dir), 133..1132 (журнал)
    let mut block_bitmap = [0u8; 4096];
    for i in 0..144 {
        block_bitmap[i] = 0xFF; // 144 * 8 = 1152 бита занято
    }
    let bb_offset = lba_offset * 512 + 2 * block_size;
    let _ = pwrite64(fd as u64, &block_bitmap, bb_offset);

    // 4. Битовая карта инодов (блок 3)
    // Заняты иноды 1..11 (биты 0..10)
    let mut inode_bitmap = [0u8; 4096];
    inode_bitmap[0] = 0xFF;
    inode_bitmap[1] = 0x07; // 8 + 3 = 11 инодов
    let ib_offset = lba_offset * 512 + 3 * block_size;
    let _ = pwrite64(fd as u64, &inode_bitmap, ib_offset);

    // 5. Инициализация таблицы инодов (блок 4..131)
    let mut itbl_buf = [0u8; 4096];

    // Инод 2: Корневой каталог (смещение в первом блоке таблицы = 1 * 256 = 256)
    let root_off = 256;
    itbl_buf[root_off..root_off + 2].copy_from_slice(&0o040755u16.to_le_bytes()); // mode: dir 0755
    itbl_buf[root_off + 4..root_off + 8].copy_from_slice(&4096u32.to_le_bytes()); // size: 4096
    itbl_buf[root_off + 26..root_off + 28].copy_from_slice(&2u16.to_le_bytes());   // links: 2
    itbl_buf[root_off + 28..root_off + 32].copy_from_slice(&8u32.to_le_bytes());   // blocks_512: 8
    itbl_buf[root_off + 32..root_off + 36].copy_from_slice(&0x0008_0000u32.to_le_bytes()); // EXTENTS_FL

    // Дерево экстентов корневого каталога: ExtentHeader (12 байт) + ExtentLeaf (12 байт)
    let ext_off = root_off + 40;
    itbl_buf[ext_off..ext_off + 2].copy_from_slice(&0xF30Au16.to_le_bytes()); // magic
    itbl_buf[ext_off + 2..ext_off + 4].copy_from_slice(&1u16.to_le_bytes());   // entries = 1
    itbl_buf[ext_off + 4..ext_off + 6].copy_from_slice(&4u16.to_le_bytes());   // max = 4
    // ExtentLeaf: логический блок 0, длина 1, физический блок 132
    itbl_buf[ext_off + 12..ext_off + 16].copy_from_slice(&0u32.to_le_bytes());   // ee_block = 0
    itbl_buf[ext_off + 16..ext_off + 18].copy_from_slice(&1u16.to_le_bytes());   // ee_len = 1
    itbl_buf[ext_off + 20..ext_off + 24].copy_from_slice(&132u32.to_le_bytes()); // ee_start = block 132

    // Инод 8: Инод журнала JBD2 (смещение = 7 * 256 = 1792)
    let j_off = 1792;
    itbl_buf[j_off..j_off + 2].copy_from_slice(&0o100600u16.to_le_bytes()); // mode: reg 0600
    itbl_buf[j_off + 4..j_off + 8].copy_from_slice(&(1000 * 4096u32).to_le_bytes()); // size = 4 МБ
    itbl_buf[j_off + 26..j_off + 28].copy_from_slice(&1u16.to_le_bytes());   // links = 1
    itbl_buf[j_off + 28..j_off + 32].copy_from_slice(&(1000 * 8u32).to_le_bytes()); // blocks_512
    itbl_buf[j_off + 32..j_off + 36].copy_from_slice(&0x0008_0000u32.to_le_bytes()); // EXTENTS_FL

    let j_ext_off = j_off + 40;
    itbl_buf[j_ext_off..j_ext_off + 2].copy_from_slice(&0xF30Au16.to_le_bytes());
    itbl_buf[j_ext_off + 2..j_ext_off + 4].copy_from_slice(&1u16.to_le_bytes());
    itbl_buf[j_ext_off + 4..j_ext_off + 6].copy_from_slice(&4u16.to_le_bytes());
    itbl_buf[j_ext_off + 12..j_ext_off + 16].copy_from_slice(&0u32.to_le_bytes());
    itbl_buf[j_ext_off + 16..j_ext_off + 18].copy_from_slice(&1000u16.to_le_bytes()); // ee_len = 1000
    itbl_buf[j_ext_off + 20..j_ext_off + 24].copy_from_slice(&133u32.to_le_bytes()); // ee_start = block 133

    let itbl_offset = lba_offset * 512 + 4 * block_size;
    let _ = pwrite64(fd as u64, &itbl_buf, itbl_offset);

    // 6. Блок корневого каталога (блок 132)
    let mut dir_buf = [0u8; 4096];
    // Запись 1: '.' (12 байт)
    dir_buf[0..4].copy_from_slice(&2u32.to_le_bytes());  // inode = 2
    dir_buf[4..6].copy_from_slice(&12u16.to_le_bytes()); // rec_len = 12
    dir_buf[6] = 1;                                     // name_len = 1
    dir_buf[7] = 2;                                     // file_type = EXT4_FT_DIR
    dir_buf[8] = b'.';

    // Запись 2: '..' (4084 байта, забирает весь оставшийся блок)
    dir_buf[12..16].copy_from_slice(&2u32.to_le_bytes());   // inode = 2
    dir_buf[16..18].copy_from_slice(&4084u16.to_le_bytes()); // rec_len = 4084
    dir_buf[18] = 2;                                        // name_len = 2
    dir_buf[19] = 2;                                        // file_type = EXT4_FT_DIR
    dir_buf[20] = b'.';
    dir_buf[21] = b'.';

    let dir_offset = lba_offset * 512 + 132 * block_size;
    let _ = pwrite64(fd as u64, &dir_buf, dir_offset);

    // 7. Суперблок журнала JBD2 (блок 133)
    let mut j_sb = [0u8; 4096];
    j_sb[0..4].copy_from_slice(&0xC036390Cu32.to_be_bytes()); // h_magic = 0xC036390C
    j_sb[4..8].copy_from_slice(&4u32.to_be_bytes());          // h_blocktype = JBD2_SUPERBLOCK_V2 (4)
    j_sb[8..12].copy_from_slice(&1u32.to_be_bytes());         // h_sequence = 1
    j_sb[12..16].copy_from_slice(&4096u32.to_be_bytes());     // s_blocksize = 4096
    j_sb[16..20].copy_from_slice(&1000u32.to_be_bytes());     // s_maxlen = 1000
    j_sb[20..24].copy_from_slice(&1u32.to_be_bytes());        // s_first = 1
    j_sb[24..28].copy_from_slice(&1u32.to_be_bytes());        // s_sequence = 1
    j_sb[28..32].copy_from_slice(&0u32.to_be_bytes());        // s_start = 0 (журнал чист)

    let j_sb_offset = lba_offset * 512 + 133 * block_size;
    let ok = pwrite64(fd as u64, &j_sb, j_sb_offset) == 4096;

    close(fd as u64);
    ok
}

/// Форматирует раздел как журналируемую файловую систему UFS2 (Soft Updates Journaling).
///
/// Записывает стандартную структуру BSD UFS2:
/// - Суперблок UFS2 (0x19540119) по смещению LBA 128 (64 КиБ) с флагом FS_SUJ
/// - Заголовок группы цилиндров (CG 0, 0x090255) с битовыми картами
/// - Таблицу инодов с корневым каталогом (inode 2)
/// - Блок корневого каталога с записями '.' и '..'
/// - Заголовок журнала SUJ (0x53554A31)
pub fn format_ufs(disk_path: &[u8], lba_offset: u64, part_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 {
        return false;
    }

    let total_frags = (part_sectors / 8).max(1024) as i64;
    let journal_lba = 1200u64;  // Смещение журнала относительно раздела (сектор 1200)
    let root_dir_sec = 2200u64; // Блок данных корневого каталога (сектор 2200 = блок 275)
    let reserved_blocks = 276i64; // Метаданные (блоки 0..275)

    // 1. Суперблок UFS2 (LBA 128 = 65536 байт от начала раздела)
    let mut sb = [0u8; 2048];
    sb[0..4].copy_from_slice(&128i32.to_le_bytes());           // fs_sblkno = 128
    sb[4..8].copy_from_slice(&0i32.to_le_bytes());             // fs_cblkno = 0
    sb[8..12].copy_from_slice(&132i32.to_le_bytes());          // fs_iblkno = 132
    sb[12..16].copy_from_slice(&(root_dir_sec as i32).to_le_bytes()); // fs_dblkno = 2200
    sb[32..40].copy_from_slice(&total_frags.to_le_bytes());    // fs_size (int64)
    sb[48..52].copy_from_slice(&4096i32.to_le_bytes());        // fs_bsize = 4096
    sb[52..56].copy_from_slice(&4096i32.to_le_bytes());        // fs_fsize = 4096
    sb[56..60].copy_from_slice(&1i32.to_le_bytes());           // fs_frag = 1
    sb[64..72].copy_from_slice(&(total_frags - reserved_blocks).to_le_bytes()); // fs_dsize
    sb[196..200].copy_from_slice(&1i32.to_le_bytes());         // fs_ncg = 1
    sb[204..208].copy_from_slice(&(total_frags as i32).to_le_bytes()); // fs_fpg
    sb[208..212].copy_from_slice(&2048i32.to_le_bytes());      // fs_ipg = 2048
    sb[1360..1368].copy_from_slice(&(journal_lba as i64).to_le_bytes()); // fs_sujfree
    sb[1368..1372].copy_from_slice(&0x0040i32.to_le_bytes());  // fs_flags = FS_SUJ
    sb[1372..1376].copy_from_slice(&0x19540119u32.to_le_bytes()); // fs_magic

    let sb_offset = (lba_offset + 128) * 512;
    if pwrite64(fd as u64, &sb, sb_offset) != 2048 {
        close(fd as u64);
        return false;
    }

    // 2. Заголовок группы цилиндров (CG 0, сектор 0)
    let mut cg = [0u8; 512];
    cg[4..8].copy_from_slice(&0i32.to_le_bytes());              // cgx = 0
    cg[16..20].copy_from_slice(&(total_frags as i32).to_le_bytes()); // ndblk
    cg[20..24].copy_from_slice(&64i32.to_le_bytes());          // niblk = 64 блока инодов
    cg[24..28].copy_from_slice(&0x090255i32.to_le_bytes());     // cg_magic = 0x090255
    cg[40..44].copy_from_slice(&((total_frags - reserved_blocks) as i32).to_le_bytes()); // nfree
    cg[44..48].copy_from_slice(&(2048 - 11i32).to_le_bytes()); // nifree

    let cg_offset = lba_offset * 512;
    let _ = pwrite64(fd as u64, &cg, cg_offset);

    // 3. Битовая карта инодов (CG + сектор 1)
    let mut imap = [0u8; 512];
    imap[0] = 0xFF;
    imap[1] = 0x07; // Заняты первые 11 инодов
    let _ = pwrite64(fd as u64, &imap, (lba_offset + 1) * 512);

    // 4. Битовая карта блоков (CG + секторы 2..127)
    // Резервируем первые 276 блоков под метаданные (секторы 0..2207)
    let mut bmap = [0u8; 512];
    for i in 0..34 {
        bmap[i] = 0xFF; // 34 * 8 = 272 блока
    }
    bmap[34] = 0x0F;    // еще 4 блока: 272 + 4 = 276 блоков занято
    let _ = pwrite64(fd as u64, &bmap, (lba_offset + 2) * 512);

    let zero = [0u8; 512];
    for s in 3..128 {
        let _ = pwrite64(fd as u64, &zero, (lba_offset + s) * 512);
    }

    // 5. Инод 2: Корневой каталог (LBA lba_offset + 132, смещение 256 байт)
    let mut root_dinode = [0u8; 256];
    root_dinode[0..2].copy_from_slice(&0o040755u16.to_le_bytes()); // mode: dir 0755
    root_dinode[2..4].copy_from_slice(&2i16.to_le_bytes());        // nlink: 2
    root_dinode[12..16].copy_from_slice(&4096u32.to_le_bytes());   // blksize = 4096
    root_dinode[16..24].copy_from_slice(&4096u64.to_le_bytes());   // size = 4096
    root_dinode[24..32].copy_from_slice(&4096u64.to_le_bytes());   // bytes = 4096
    root_dinode[112..120].copy_from_slice(&(root_dir_sec as i64).to_le_bytes()); // di_db[0]

    let inode_offset = (lba_offset + 132) * 512 + 256;
    let _ = pwrite64(fd as u64, &root_dinode, inode_offset);

    // 6. Блок данных корневого каталога (LBA root_dir_sec = 2200)
    let mut dir_buf = [0u8; 4096];
    // Запись 1: '.'
    dir_buf[0..4].copy_from_slice(&2u32.to_le_bytes());  // d_ino = 2
    dir_buf[4..6].copy_from_slice(&12u16.to_le_bytes()); // d_reclen = 12
    dir_buf[6] = 4;                                     // d_type = DT_DIR (4)
    dir_buf[7] = 1;                                     // d_namlen = 1
    dir_buf[8] = b'.';

    // Запись 2: '..'
    dir_buf[12..16].copy_from_slice(&2u32.to_le_bytes());   // d_ino = 2
    dir_buf[16..18].copy_from_slice(&4084u16.to_le_bytes()); // d_reclen = 4084
    dir_buf[18] = 4;                                        // d_type = DT_DIR (4)
    dir_buf[19] = 2;                                        // d_namlen = 2
    dir_buf[20] = b'.';
    dir_buf[21] = b'.';

    let _ = pwrite64(fd as u64, &dir_buf, (lba_offset + root_dir_sec) * 512);

    // 7. Заголовок журнала SUJ (LBA journal_lba = 1200)
    let mut j_hdr = [0u8; 512];
    j_hdr[0..4].copy_from_slice(&0x53554A31u32.to_le_bytes()); // magic = "SUJ1"
    j_hdr[4..8].copy_from_slice(&1u32.to_le_bytes());          // seq = 1
    let ok = pwrite64(fd as u64, &j_hdr, (lba_offset + journal_lba) * 512) == 512;

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

    // Считываем текущий сектор 0 (MBR) с диска целиком для гарантированного сохранения таблицы разделов
    let mut current_mbr = [0u8; 512];
    let r = pread64(fd_disk as u64, &mut current_mbr, 0);
    if r == 512 {
        boot_sector[440..510].copy_from_slice(&current_mbr[440..510]);
    }

    let stage2_size_a: u16 = 10752;
    let stage2_size_b: u16 = 10752;
    let stage2_loc_a: u64 = 0x200;
    let stage2_loc_b: u64 = 0x2c00;

    boot_sector[420..422].copy_from_slice(&stage2_size_a.to_le_bytes());
    boot_sector[422..424].copy_from_slice(&stage2_size_b.to_le_bytes());
    boot_sector[424..432].copy_from_slice(&stage2_loc_a.to_le_bytes());
    boot_sector[432..440].copy_from_slice(&stage2_loc_b.to_le_bytes());

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
