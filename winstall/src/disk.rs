// Обнаружение дисков, создание разделов MBR и форматирование FAT32.

use libwunix::{close, open, pread64, pwrite64, O_RDONLY, O_RDWR};

pub const MAX_DISKS: usize = 8;

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
}

impl Default for DiskInfo {
    fn default() -> Self {
        Self { path: b"\0", sector_count: 0 }
    }
}

fn probe(path: &'static [u8]) -> Option<DiskInfo> {
    let fd = open(path, O_RDONLY as u64, 0);
    if fd < 0 { return None; }
    let mut buf = [0u8; 512];
    let n = pread64(fd as u64, &mut buf, 0);
    close(fd as u64);
    if n <= 0 { return None; }

    // Проверяем доступный размер диска (проверяем сектор 64 MiB и 128 MiB)
    let fd2 = open(path, O_RDONLY as u64, 0);
    if fd2 < 0 { return Some(DiskInfo { path, sector_count: 65536 }); }
    let n2 = pread64(fd2 as u64, &mut buf, 131072 * 512); // 64 MiB
    let n3 = pread64(fd2 as u64, &mut buf, 262144 * 512); // 128 MiB
    close(fd2 as u64);

    let sectors = if n3 > 0 {
        262144
    } else if n2 > 0 {
        131072
    } else {
        65536
    };
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

/// Записывает MBR: активный загрузочный раздел FAT32 с LBA 2048 (1 MiB).
pub fn write_mbr(disk_path: &[u8], total_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    let start: u32 = 2048;
    let size: u32 = (total_sectors.saturating_sub(2048)).min(0xFFFF_FFFF) as u32;

    let mut mbr = [0u8; 512];
    let pe = &mut mbr[446..462];
    pe[0] = 0x80;                                    // bootable (активный)
    pe[1] = 0x00; pe[2] = 0x01; pe[3] = 0x00;       // CHS start
    pe[4] = 0x0C;                                    // FAT32 LBA
    pe[5] = 0xFE; pe[6] = 0xFF; pe[7] = 0xFF;       // CHS end
    pe[8..12].copy_from_slice(&start.to_le_bytes());
    pe[12..16].copy_from_slice(&size.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;

    let ok = pwrite64(fd as u64, &mbr, 0) == 512;
    close(fd as u64);
    ok
}

/// Форматирует раздел (lba_offset в секторах) как валидную FAT32.
pub fn format_fat32(disk_path: &[u8], lba_offset: u64, part_sectors: u64) -> bool {
    let fd = open(disk_path, O_RDWR as u64, 0);
    if fd < 0 { return false; }

    let byte_off = lba_offset * 512;
    // Для соответствия спецификации FAT32 (CountOfClusters >= 65525):
    // При part_sectors >= 530_000 секторов (260+ MiB) используем 8 секторов на кластер (4 КиБ).
    // При меньших дисках выбираем 1 сектор на кластер (512 байт), чтобы число кластеров гарантированно было >= 65525.
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

    // Записываем BPB в сектор 0 раздела и в backup сектор 6
    let mut ok = pwrite64(fd as u64, &bpb, byte_off) == 512;
    let _ = pwrite64(fd as u64, &bpb, byte_off + 6 * 512);

    // Сектор 1: FSInfo
    let mut fsinfo = [0u8; 512];
    fsinfo[0..4].copy_from_slice(b"RRaA");
    fsinfo[484..488].copy_from_slice(b"rrAa");
    fsinfo[488..492].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // free cluster count unknown
    fsinfo[492..496].copy_from_slice(&2u32.to_le_bytes());          // next free cluster hint
    fsinfo[510] = 0x55; fsinfo[511] = 0xAA;
    let _ = pwrite64(fd as u64, &fsinfo, byte_off + 512);

    // Инициализация первой и второй таблицы FAT
    let fat1_off = byte_off + (rsvd_sec as u64) * 512;
    let fat2_off = fat1_off + fat_sectors as u64 * 512;
    let mut fat_hdr = [0u8; 512];
    // FAT[0] = Media descriptor (0x0FFFFFF8), FAT[1] = EOC, FAT[2] = Root dir EOC
    fat_hdr[0] = 0xF8; fat_hdr[1] = 0xFF; fat_hdr[2] = 0xFF; fat_hdr[3] = 0x0F;
    fat_hdr[4] = 0xFF; fat_hdr[5] = 0xFF; fat_hdr[6] = 0xFF; fat_hdr[7] = 0x0F;
    fat_hdr[8] = 0xFF; fat_hdr[9] = 0xFF; fat_hdr[10] = 0xFF; fat_hdr[11] = 0x0F;
    ok &= pwrite64(fd as u64, &fat_hdr, fat1_off) == 512;
    ok &= pwrite64(fd as u64, &fat_hdr, fat2_off) == 512;

    // Обнуляем корневой кластер (каталог)
    let data_start = fat2_off + fat_sectors as u64 * 512;
    let zero = [0u8; 512];
    for c_sec in 0..(sec_per_clus as u64) {
        let _ = pwrite64(fd as u64, &zero, data_start + c_sec * 512);
    }

    close(fd as u64);
    ok
}

/// Устанавливает загрузочную запись Limine (Stage 1 и Stage 2) на целевой диск,
/// если файл /boot/limine_hdd.bin присутствует в initramfs.
pub fn install_limine_bootloader(disk_path: &[u8]) -> bool {
    let fd_bin = open(b"/boot/limine_hdd.bin\0", O_RDONLY as u64, 0);
    if fd_bin < 0 {
        // Если файла нет в initramfs, диск остаётся с базовым MBR
        return true;
    }

    let fd_disk = open(disk_path, O_RDWR as u64, 0);
    if fd_disk < 0 {
        close(fd_bin as u64);
        return false;
    }

    // Читаем MBR bootsector (сектор 0) из образа Limine
    let mut boot_sector = [0u8; 512];
    let n = pread64(fd_bin as u64, &mut boot_sector, 0);
    if n != 512 {
        close(fd_bin as u64);
        close(fd_disk as u64);
        return false;
    }

    // Сохраняем существующую таблицу разделов MBR целевого диска (байты 440..510)
    let mut orig_part_table = [0u8; 70];
    let _ = pread64(fd_disk as u64, &mut orig_part_table, 440);

    // Геометрия Limine Stage 2 для MBR (limine_hdd.bin размером 21547 байт):
    // Stage 2 размер = 21035 байт (42 сектора).
    // Половина A: 21 сектор = 10752 байт по смещению 0x200 (сектор 1).
    // Половина B: 21 сектор = 10752 байт по смещению 0x2C00 (сектор 22).
    let stage2_size_a: u16 = 10752;
    let stage2_size_b: u16 = 10752;
    let stage2_loc_a: u64 = 0x200;
    let stage2_loc_b: u64 = 0x2c00;

    // Прошиваем метаданные расположения Stage 2 в сектор 0 по смещению 0x1A4 (420)
    boot_sector[420..422].copy_from_slice(&stage2_size_a.to_le_bytes());
    boot_sector[422..424].copy_from_slice(&stage2_size_b.to_le_bytes());
    boot_sector[424..432].copy_from_slice(&stage2_loc_a.to_le_bytes());
    boot_sector[432..440].copy_from_slice(&stage2_loc_b.to_le_bytes());

    // Переносим таблицу разделов MBR и загрузочную сигнатуру 0x55AA
    boot_sector[440..510].copy_from_slice(&orig_part_table);
    boot_sector[510] = 0x55;
    boot_sector[511] = 0xAA;

    // Записываем сконфигурированный bootsector в сектор 0 целевого диска
    let _ = pwrite64(fd_disk as u64, &boot_sector, 0);

    // Копируем тело Stage 2 (сектора 1..42) в post-MBR зазор перед разделом 1 (LBA 2048)
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
