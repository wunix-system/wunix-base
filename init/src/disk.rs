//! Проверка и монтирование постоянного диска установленной системы Wunix.

use libwunix::{
    close, mkdir, mount, open, pread64, pwrite64, umount2, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY,
};

/// Проверяет, является ли окружение установленной системой.
/// Если система найдена на диске, монтирует /dev/vda1 в /mnt и синхронизирует учетные записи.
pub fn check_installed() -> bool {
    let fd = open(b"/etc/installed\0", O_RDONLY as u64, 0);
    if fd >= 0 {
        close(fd as u64);
        return true;
    }

    // Проверяем доступные дисковые разделы
    let candidate_devs: &[&[u8]] = &[
        b"/dev/vda3\0",
        b"/dev/sda3\0",
        b"/dev/vda1\0",
        b"/dev/sda1\0",
        b"/dev/vda2\0",
        b"/dev/sda2\0",
        b"/dev/vda\0",
        b"/dev/sda\0",
    ];

    let _ = mkdir(b"/mnt\0", 0o755);

    for dev in candidate_devs {
        for fstype in &[b"ufs\0".as_ref(), b"ext4\0", b"fat32\0", b"\0"] {
            if mount(*dev, b"/mnt\0", *fstype, 0, b"\0") >= 0 {
                let inst_fd = open(b"/mnt/etc/installed\0", O_RDONLY as u64, 0);
                if inst_fd >= 0 {
                    close(inst_fd as u64);
                    // Синхронизируем учетные записи и системные конфигурации с установленного диска
                    sync_file(b"/mnt/etc/passwd\0", b"/etc/passwd\0", 0o644);
                    sync_file(b"/mnt/etc/shadow\0", b"/etc/shadow\0", 0o600);
                    sync_file(b"/mnt/etc/group\0", b"/etc/group\0", 0o644);
                    sync_file(b"/mnt/etc/hostname\0", b"/etc/hostname\0", 0o644);
                    sync_file(b"/mnt/etc/network.conf\0", b"/etc/network.conf\0", 0o644);
                    sync_file(b"/mnt/etc/resolv.conf\0", b"/etc/resolv.conf\0", 0o644);
                    sync_file(b"/mnt/etc/installed\0", b"/etc/installed\0", 0o644);
                    return true;
                } else {
                    let _ = umount2(b"/mnt\0", 0);
                }
            }
        }
    }

    false
}

fn sync_file(src: &[u8], dst: &[u8], mode: u64) {
    let sfd = open(src, O_RDONLY as u64, 0);
    if sfd < 0 {
        return;
    }
    let dfd = open(dst, (O_WRONLY | O_CREAT | O_TRUNC) as u64, mode);
    if dfd < 0 {
        close(sfd as u64);
        return;
    }
    let mut buf = [0u8; 1024];
    let mut off = 0u64;
    loop {
        let n = pread64(sfd as u64, &mut buf, off);
        if n <= 0 {
            break;
        }
        pwrite64(dfd as u64, &buf[..n as usize], off);
        off += n as u64;
    }
    close(sfd as u64);
    close(dfd as u64);
}
