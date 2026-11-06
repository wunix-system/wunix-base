// Копирование Wunix base и установка системы на смонтированный раздел.

use crate::disk::install_limine_bootloader;
use crate::net::{apply_network_config, NetConfig};

use libwunix::{
    close, hash_password, mkdir, mount, open, pread64, pwrite64, umount2,
    O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY,
};

/// Список системных утилит Wunix Base для копирования на диск.
/// ВНИМАНИЕ: winstall здесь ИСКЛЮЧЕН — установщик на диске не нужен!
pub const INSTALL_BINARIES: &[(&[u8], &[u8])] = &[
    (b"/bin/init\0",       b"/mnt/bin/init\0"),
    (b"/bin/wsh\0",        b"/mnt/bin/wsh\0"),
    (b"/bin/echo\0",       b"/mnt/bin/echo\0"),
    (b"/bin/cat\0",        b"/mnt/bin/cat\0"),
    (b"/bin/ls\0",         b"/mnt/bin/ls\0"),
    (b"/bin/clear\0",      b"/mnt/bin/clear\0"),
    (b"/bin/uname\0",      b"/mnt/bin/uname\0"),
    (b"/bin/pwd\0",        b"/mnt/bin/pwd\0"),
    (b"/bin/touch\0",      b"/mnt/bin/touch\0"),
    (b"/bin/rm\0",         b"/mnt/bin/rm\0"),
    (b"/bin/mkdir\0",      b"/mnt/bin/mkdir\0"),
    (b"/bin/rmdir\0",      b"/mnt/bin/rmdir\0"),
    (b"/bin/ln\0",         b"/mnt/bin/ln\0"),
    (b"/bin/readlink\0",   b"/mnt/bin/readlink\0"),
    (b"/bin/top\0",        b"/mnt/bin/top\0"),
    (b"/bin/free\0",       b"/mnt/bin/free\0"),
    (b"/bin/ping\0",       b"/mnt/bin/ping\0"),
    (b"/bin/ip\0",         b"/mnt/bin/ip\0"),
    (b"/bin/curl\0",       b"/mnt/bin/curl\0"),
    (b"/bin/host\0",       b"/mnt/bin/host\0"),
    (b"/bin/ss\0",         b"/mnt/bin/ss\0"),
    (b"/bin/lsblk\0",      b"/mnt/bin/lsblk\0"),
    (b"/bin/fdisk\0",      b"/mnt/bin/fdisk\0"),
    (b"/bin/mount\0",      b"/mnt/bin/mount\0"),
    (b"/bin/umount\0",     b"/mnt/bin/umount\0"),
    (b"/bin/man\0",        b"/mnt/bin/man\0"),
    (b"/bin/lspci\0",      b"/mnt/bin/lspci\0"),
    (b"/bin/lshw\0",       b"/mnt/bin/lshw\0"),
    (b"/bin/lsdev\0",      b"/mnt/bin/lsdev\0"),
    (b"/bin/lsmod\0",      b"/mnt/bin/lsmod\0"),
    (b"/bin/modinfo\0",    b"/mnt/bin/modinfo\0"),
    (b"/bin/hwclock\0",    b"/mnt/bin/hwclock\0"),
    (b"/bin/fbset\0",      b"/mnt/bin/fbset\0"),
    (b"/bin/date\0",       b"/mnt/bin/date\0"),
    (b"/bin/time\0",       b"/mnt/bin/time\0"),
    (b"/bin/timedatectl\0",b"/mnt/bin/timedatectl\0"),
    (b"/bin/shutdown\0",   b"/mnt/bin/shutdown\0"),
    (b"/bin/reboot\0",     b"/mnt/bin/reboot\0"),
    (b"/bin/whoami\0",     b"/mnt/bin/whoami\0"),
    (b"/bin/id\0",         b"/mnt/bin/id\0"),
    (b"/bin/chmod\0",      b"/mnt/bin/chmod\0"),
    (b"/bin/chown\0",      b"/mnt/bin/chown\0"),
    (b"/bin/passwd\0",     b"/mnt/bin/passwd\0"),
    (b"/bin/useradd\0",    b"/mnt/bin/useradd\0"),
    (b"/bin/userdel\0",    b"/mnt/bin/userdel\0"),
    (b"/bin/su\0",         b"/mnt/bin/su\0"),
    (b"/bin/sudo\0",       b"/mnt/bin/sudo\0"),
    (b"/bin/login\0",      b"/mnt/bin/login\0"),
    (b"/bin/sudoadd\0",    b"/mnt/bin/sudoadd\0"),
    (b"/bin/nano\0",       b"/mnt/bin/nano\0"),
    (b"/bin/ssh\0",        b"/mnt/bin/ssh\0"),
    (b"/bin/sshd\0",       b"/mnt/bin/sshd\0"),
    (b"/bin/openssl\0",    b"/mnt/bin/openssl\0"),
    (b"/bin/ntpdate\0",    b"/mnt/bin/ntpdate\0"),
    (b"/bin/timesyncd\0",  b"/mnt/bin/timesyncd\0"),
    (b"/bin/system\0",     b"/mnt/bin/system\0"),
    (b"/bin/nslookup\0",   b"/mnt/bin/nslookup\0"),
];

pub const INSTALL_BOOT_FILES: &[(&[u8], &[u8])] = &[
    (b"/boot/wunix-kernel\0",   b"/mnt/boot/wunix-kernel\0"),
    (b"/boot/limine.conf\0",    b"/mnt/boot/limine.conf\0"),
    (b"/boot/limine-bios.sys\0",b"/mnt/boot/limine-bios.sys\0"),
];

pub const INSTALL_ETC_FILES: &[(&[u8], &[u8])] = &[
    (b"/etc/os-release\0",                 b"/mnt/etc/os-release\0"),
    (b"/etc/locale\0",                     b"/mnt/etc/locale\0"),
    (b"/etc/hosts\0",                      b"/mnt/etc/hosts\0"),
    (b"/etc/resolv.conf\0",                b"/mnt/etc/resolv.conf\0"),
    (b"/etc/timesyncd.conf\0",             b"/mnt/etc/timesyncd.conf\0"),
    (b"/etc/ntp.conf\0",                   b"/mnt/etc/ntp.conf\0"),
    (b"/etc/ssl/certs/ca-certificates.crt\0", b"/mnt/etc/ssl/certs/ca-certificates.crt\0"),
    (b"/etc/ssl/openssl.cnf\0",            b"/mnt/etc/ssl/openssl.cnf\0"),
    (b"/etc/ssh/ssh_config\0",             b"/mnt/etc/ssh/ssh_config\0"),
    (b"/etc/ssh/sshd_config\0",            b"/mnt/etc/ssh/sshd_config\0"),
    (b"/etc/ssh/ssh_host_ed25519_key\0",      b"/mnt/etc/ssh/ssh_host_ed25519_key\0"),
    (b"/etc/ssh/ssh_host_ed25519_key.pub\0",  b"/mnt/etc/ssh/ssh_host_ed25519_key.pub\0"),
    (b"/etc/ssh/ssh_host_rsa_key\0",          b"/mnt/etc/ssh/ssh_host_rsa_key\0"),
    (b"/etc/ssh/ssh_host_rsa_key.pub\0",      b"/mnt/etc/ssh/ssh_host_rsa_key.pub\0"),
];

pub const INSTALL_EXTRA_FILES: &[(&[u8], &[u8])] = &[];

pub fn mount_root_partition(disk_path: &[u8], root_part_num: u8) -> bool {
    let _ = mkdir(b"/mnt\0", 0o755);

    let mut part_path = [0u8; 64];
    let mut len = 0;
    while len < disk_path.len() && disk_path[len] != 0 && len < 60 {
        part_path[len] = disk_path[len];
        len += 1;
    }
    part_path[len] = b'0' + root_part_num;
    part_path[len + 1] = 0;

    if mount(&part_path[..len + 2], b"/mnt\0", b"fat32\0", 0, b"\0") >= 0 {
        return true;
    }

    // Если раздел не зарегистрирован, пробуем смонтировать сам диск
    mount(disk_path, b"/mnt\0", b"fat32\0", 0, b"\0") >= 0
}

/// Копирует ядро и конфигурации загрузчика на выделенный раздел /boot (если он задан)
pub fn copy_boot_files_to_boot_partition(disk_path: &[u8], boot_part_num: u8, root_dev_str: &str) {
    let _ = mkdir(b"/mnt_boot\0", 0o755);
    let mut part_path = [0u8; 64];
    let mut len = 0;
    while len < disk_path.len() && disk_path[len] != 0 && len < 60 {
        part_path[len] = disk_path[len];
        len += 1;
    }
    part_path[len] = b'0' + boot_part_num;
    part_path[len + 1] = 0;

    if mount(&part_path[..len + 2], b"/mnt_boot\0", b"fat32\0", 0, b"\0") >= 0 {
        let _ = mkdir(b"/mnt_boot/boot\0", 0o755);
        copy_file(b"/boot/wunix-kernel\0", b"/mnt_boot/boot/wunix-kernel\0", 0o644);
        copy_file(b"/boot/wunix-kernel\0", b"/mnt_boot/wunix-kernel\0", 0o644);
        copy_file(b"/boot/limine-bios.sys\0", b"/mnt_boot/limine-bios.sys\0", 0o644);
        copy_file(b"/boot/limine-bios.sys\0", b"/mnt_boot/boot/limine-bios.sys\0", 0o644);

        let mut conf_buf = [0u8; 512];
        let mut cpos = 0;
        cpos = ap(&mut conf_buf, cpos, b"timeout: 0\n\n/Wunix OS\n    protocol: limine\n    kernel_path: boot():/boot/wunix-kernel\n    cmdline: root=");
        cpos = ap(&mut conf_buf, cpos, root_dev_str.as_bytes());
        cpos = ap(&mut conf_buf, cpos, b" lang=ru mode=disk\n");
        let limine_conf_content = &conf_buf[..cpos];
        write_file(b"/mnt_boot/boot/limine.conf\0", limine_conf_content, 0o644);
        write_file(b"/mnt_boot/limine.conf\0", limine_conf_content, 0o644);

        let _ = umount2(b"/mnt_boot\0", 0);
    }
}

pub fn unmount_target() {
    let _ = umount2(b"/mnt\0", 0);
}

fn ensure_dir(p: &[u8]) {
    let _ = mkdir(p, 0o755);
}

fn copy_file(src: &[u8], dst: &[u8], mode: u64) -> bool {
    let sfd = open(src, O_RDONLY as u64, 0);
    if sfd < 0 { return false; }
    let dfd = open(dst, (O_WRONLY | O_CREAT | O_TRUNC) as u64, mode);
    if dfd < 0 {
        close(sfd as u64);
        return false;
    }
    let mut buf = [0u8; 65536];
    let mut off = 0u64;
    loop {
        let n = pread64(sfd as u64, &mut buf, off);
        if n <= 0 { break; }
        pwrite64(dfd as u64, &buf[..n as usize], off);
        off += n as u64;
    }
    close(sfd as u64);
    close(dfd as u64);
    true
}

fn write_file(path: &[u8], data: &[u8], mode: u64) -> bool {
    let fd = open(path, (O_WRONLY | O_CREAT | O_TRUNC) as u64, mode);
    if fd < 0 { return false; }
    let ok = pwrite64(fd as u64, data, 0) == data.len() as i64;
    close(fd as u64);
    ok
}

fn ap(buf: &mut [u8], pos: usize, s: &[u8]) -> usize {
    let end = (pos + s.len()).min(buf.len());
    buf[pos..end].copy_from_slice(&s[..end - pos]);
    end
}

fn make_salt(out: &mut [u8; 8]) {
    let addr = out.as_ptr() as u64;
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789./";
    for i in 0..8 {
        out[i] = chars[((addr.wrapping_add(i as u64 * 0x9E3779B9)) >> 4 & 63) as usize];
    }
}

pub fn total_install_items() -> usize {
    INSTALL_BINARIES.len() + INSTALL_BOOT_FILES.len() + INSTALL_ETC_FILES.len() + INSTALL_EXTRA_FILES.len() + 10
}

/// Выполняет установку системы Wunix на смонтированный диск
pub fn install_base<F>(
    disk_path: &[u8],
    root_part_num: u8,
    boot_part_num: u8,
    hostname: &str,
    timezone: &str,
    admin_user: &str,
    password: &[u8],
    net_cfg: &NetConfig,
    mut progress: F,
) -> bool
where
    F: FnMut(u8, &str, usize, usize),
{
    let total = total_install_items();
    let mut done = 0;

    progress(2, "Монтирование диска...", done, total);
    if !mount_root_partition(disk_path, root_part_num) {
        return false;
    }
    done += 1;

    progress(5, "Создание структуры каталогов...", done, total);
    for d in &[
        b"/mnt/bin\0".as_ref(), b"/mnt/etc\0", b"/mnt/tmp\0",
        b"/mnt/root\0", b"/mnt/home\0", b"/mnt/mnt\0",
        b"/mnt/boot\0", b"/mnt/dev\0",  b"/mnt/proc\0", b"/mnt/sys\0",
        b"/mnt/etc/ssl\0", b"/mnt/etc/ssl/certs\0",
        b"/mnt/etc/ssh\0", b"/mnt/etc/services\0", b"/mnt/etc/services/enabled\0",
        b"/mnt/var\0", b"/mnt/var/empty\0", b"/mnt/var/log\0", b"/mnt/var/log/services\0",
        b"/mnt/run\0", b"/mnt/run/services\0",
        b"/mnt/usr\0", b"/mnt/usr/lib\0",
        b"/mnt/usr/include\0", b"/mnt/usr/share\0",
    ] {
        ensure_dir(d);
    }
    let _ = libwunix::chmod(b"/mnt/tmp\0", 0o1777);
    let _ = libwunix::chmod(b"/mnt/root\0", 0o700);
    done += 1;

    // 1. Копирование системных бинарных файлов (/bin) - БЕЗ winstall!
    for (src, dst) in INSTALL_BINARIES {
        let s = *src;
        let start = s.iter().rposition(|&b| b == b'/').map(|p| p + 1).unwrap_or(0);
        let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
        let name = core::str::from_utf8(&s[start..end]).unwrap_or("bin");
        let pct = (10 + (done * 50 / total)).min(60) as u8;
        progress(pct, name, done, total);

        let mode = if name == "sudo" || name == "su" || name == "passwd" || name == "sudoadd" {
            0o4755
        } else {
            0o755
        };
        copy_file(src, dst, mode);
        let _ = libwunix::chmod(dst, mode as u32);
        done += 1;
    }

    // 2. Копирование файлов ядра и загрузчика (/boot)
    progress(62, "Установка ядра и загрузчика (/boot)...", done, total);
    for (src, dst) in INSTALL_BOOT_FILES {
        copy_file(src, dst, 0o644);
        done += 1;
    }

    // Создаем правильный limine.conf для жесткого диска
    let mut part_path = [0u8; 64];
    let mut plen = 0;
    while plen < disk_path.len() && disk_path[plen] != 0 && plen < 60 {
        part_path[plen] = disk_path[plen];
        plen += 1;
    }
    part_path[plen] = b'0' + root_part_num;
    let root_dev_str = core::str::from_utf8(&part_path[..plen + 1]).unwrap_or("/dev/vda1");

    let mut conf_buf = [0u8; 512];
    let mut cpos = 0;
    cpos = ap(&mut conf_buf, cpos, b"timeout: 0\n\n/Wunix OS\n    protocol: limine\n    kernel_path: boot():/boot/wunix-kernel\n    cmdline: root=");
    cpos = ap(&mut conf_buf, cpos, root_dev_str.as_bytes());
    cpos = ap(&mut conf_buf, cpos, b" lang=ru mode=disk\n");
    let limine_conf_content = &conf_buf[..cpos];
    write_file(b"/mnt/boot/limine.conf\0", limine_conf_content, 0o644);

    // 3. Базовые конфигурационные файлы (/etc)
    progress(68, "Копирование конфигураций (/etc)...", done, total);
    for (src, dst) in INSTALL_ETC_FILES {
        copy_file(src, dst, 0o644);
        done += 1;
    }

    for (src, dst) in INSTALL_EXTRA_FILES {
        copy_file(src, dst, 0o644);
        done += 1;
    }

    // Автозапуск системных служб Wunix (timesyncd, network, ssh)
    const TIMESYNCD_CFG: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd1\x81\xd0\xb8\xd0\xbd\xd1\x85\xd1\x80\xd0\xbe\xd0\xbd\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd0\xb8 \xd0\xb2\xd1\x80\xd0\xb5\xd0\xbc\xd0\xb5\xd0\xbd\xd0\xb8 Wunix NTP\n\
description = \xd0\xa1\xd0\xb8\xd0\xbd\xd1\x85\xd1\x80\xd0\xbe\xd0\xbd\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd1\x8f \xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xbd\xd1\x8b\xd1\x85 \xd1\x87\xd0\xb0\xd1\x81\xd0\xbe\xd0\xb2 \xd0\xbf\xd0\xbe \xd0\xbf\xd1\x80\xd0\xbe\xd1\x82\xd0\xbe\xd0\xba\xd0\xbe\xd0\xbb\xd1\x83 NTP\n\
command = /bin/timesyncd -1\n\
autostart = yes\n\
restart = on-failure\n\
user = root\n\
directory = /\n";

    const NETWORK_CFG: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd0\xbd\xd0\xb0\xd1\x81\xd1\x82\xd1\x80\xd0\xbe\xd0\xb9\xd0\xba\xd0\xb8 \xd1\x81\xd0\xb5\xd1\x82\xd0\xb8\n\
description = \xd0\x9f\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0 \xd0\xb8 \xd0\xb8\xd0\xbd\xd0\xb8\xd1\x86\xd0\xb8\xd0\xb0\xd0\xbb\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd1\x8f \xd1\x81\xd0\xb5\xd1\x82\xd0\xb5\xd0\xb2\xd1\x8b\xd1\x85 \xd0\xb8\xd0\xbd\xd1\x82\xd0\xb5\xd1\x80\xd1\x84\xd0\xb5\xd0\xb9\xd1\x81\xd0\xbe\xd0\xb2\n\
command = /bin/ip addr\n\
autostart = yes\n\
restart = no\n\
user = root\n\
directory = /\n";

    const SSH_CFG: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 OpenSSH\n\
description = \xd0\xa1\xd0\xb5\xd1\x82\xd0\xb5\xd0\xb2\xd0\xbe\xd0\xb9 \xd1\x81\xd0\xb5\xd1\x80\xd0\xb2\xd0\xb5\xd1\x80 \xd1\x83\xd0\xb4\xd0\xb0\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xbd\xd0\xbe\xd0\xb3\xd0\xbe \xd0\xb4\xd0\xbe\xd1\x81\xd1\x82\xd1\x83\xd0\xbf\xd0\xb0 SSH\n\
command = /bin/sshd -D\n\
autostart = no\n\
restart = always\n\
user = root\n\
directory = /\n";

    write_file(b"/mnt/etc/services/timesyncd.conf\0", TIMESYNCD_CFG, 0o644);
    write_file(b"/mnt/etc/services/network.conf\0", NETWORK_CFG, 0o644);
    write_file(b"/mnt/etc/services/ssh.conf\0", SSH_CFG, 0o644);
    write_file(b"/mnt/etc/services/sshd.conf\0", SSH_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/timesyncd.conf\0", TIMESYNCD_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/network.conf\0", NETWORK_CFG, 0o644);

    // 4. Имя хоста (/etc/hostname)
    progress(72, "Настройка имени компьютера...", done, total);
    {
        let mut buf = [0u8; 128];
        let l = hostname.len().min(126);
        buf[..l].copy_from_slice(hostname.as_bytes());
        buf[l] = b'\n';
        write_file(b"/mnt/etc/hostname\0", &buf[..l + 1], 0o644);
    }
    done += 1;

    // 5. Часовой пояс (/etc/timezone)
    progress(74, "Настройка часового пояса...", done, total);
    {
        let mut buf = [0u8; 64];
        let l = timezone.len().min(62);
        buf[..l].copy_from_slice(timezone.as_bytes());
        buf[l] = b'\n';
        write_file(b"/mnt/etc/timezone\0", &buf[..l + 1], 0o644);
    }
    done += 1;

    // 6. Сетевая конфигурация
    progress(76, "Применение сетевой конфигурации...", done, total);
    apply_network_config(net_cfg, hostname);
    done += 1;

    // 7. Создание учетных записей (root и admin)
    progress(80, "Создание пользователей (root, sudo)...", done, total);
    {
        let mut hash_buf = [0u8; 128];
        let hash_len = if !password.is_empty() {
            if let Ok(pass_str) = core::str::from_utf8(password) {
                let mut salt = [0u8; 8];
                make_salt(&mut salt);
                let salt_str = core::str::from_utf8(&salt).unwrap_or("wunixslt");
                hash_password(pass_str, salt_str, &mut hash_buf)
            } else { 0 }
        } else { 0 };

        let hash = &hash_buf[..hash_len];

        // /etc/passwd
        let mut pb = [0u8; 512];
        let mut pos = ap(&mut pb, 0, b"root:x:0:0:Superuser:/root:/bin/wsh\n");
        if !admin_user.is_empty() {
            pos = ap(&mut pb, pos, admin_user.as_bytes());
            pos = ap(&mut pb, pos, b":x:1000:1000:Admin:/home/");
            pos = ap(&mut pb, pos, admin_user.as_bytes());
            pos = ap(&mut pb, pos, b":/bin/wsh\n");
        }
        write_file(b"/mnt/etc/passwd\0", &pb[..pos], 0o644);

        // /etc/shadow
        let mut sb = [0u8; 768];
        let mut pos = ap(&mut sb, 0, b"root:");
        pos = ap(&mut sb, pos, hash);
        pos = ap(&mut sb, pos, b":19000:0:99999:7:::\n");
        if !admin_user.is_empty() {
            pos = ap(&mut sb, pos, admin_user.as_bytes());
            pos = ap(&mut sb, pos, b":");
            pos = ap(&mut sb, pos, hash);
            pos = ap(&mut sb, pos, b":19000:0:99999:7:::\n");
        }
        write_file(b"/mnt/etc/shadow\0", &sb[..pos], 0o600);

        // /etc/group
        let mut gb = [0u8; 256];
        let mut pos = ap(&mut gb, 0, b"root:x:0:root\nwheel:x:10:root");
        if !admin_user.is_empty() {
            pos = ap(&mut gb, pos, b",");
            pos = ap(&mut gb, pos, admin_user.as_bytes());
        }
        pos = ap(&mut gb, pos, b"\n");
        write_file(b"/mnt/etc/group\0", &gb[..pos], 0o644);
    }
    done += 1;

    // 8. Создание домашнего каталога администратора
    progress(86, "Создание домашних каталогов...", done, total);
    if !admin_user.is_empty() {
        let mut hd = [0u8; 64];
        let mut pos = ap(&mut hd, 0, b"/mnt/home/");
        pos = ap(&mut hd, pos, admin_user.as_bytes());
        hd[pos] = 0;
        ensure_dir(&hd[..pos + 1]);
        let _ = libwunix::chown(&hd[..pos + 1], 1000, 1000);
        let _ = libwunix::chmod(&hd[..pos + 1], 0o750);
    }
    done += 1;

    // 9. Маркер установленной системы
    progress(90, "Фиксация флага установки...", done, total);
    write_file(
        b"/mnt/etc/installed\0",
        b"WUNIX_INSTALLED=1\nINSTALL_STAGE=11\nRELEASE=1.0\n",
        0o644,
    );
    done += 1;

    // 10. Размонтирование файловой системы
    progress(94, "Сброс дискового кэша и размонтирование /mnt...", done, total);
    unmount_target();
    done += 1;

    // 10b. Если выделен отдельный раздел /boot, копируем файлы ядра и загрузчика и туда
    if boot_part_num != 0 && boot_part_num != root_part_num {
        copy_boot_files_to_boot_partition(disk_path, boot_part_num, root_dev_str);
    }

    // 11. Установка загрузчика Limine в MBR
    progress(98, "Установка загрузчика Limine в MBR диска...", done, total);
    let _ = install_limine_bootloader(disk_path);
    done += 1;

    progress(100, "Установка Wunix OS успешно завершена!", done, total);
    true
}
