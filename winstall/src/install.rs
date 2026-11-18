// Установка Wunix base и прикладных пакетов через WSP и WPM на смонтированный раздел.

use crate::disk::install_limine_bootloader;
use crate::net::{apply_network_config, NetConfig};

use libwunix::{
    close, hash_password, mkdir, mount, open, pread64, pwrite64, symlink, umount2, unlink,
    O_APPEND, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY,
};

/// Список компонентов базовой системы и ядра Wunix, устанавливаемых через WSP
pub const SYSTEM_PACKAGES: &[&str] = &[
    "wunix-kernel",
    "libwunix",
    "libpackwunix",
    "libcwunix",
    "wunix-base",
    "coreutils",
    "netutils",
    "sysutils",
    "authutils",
    "wspd",
    "openssl",
    "ncurses",
    "zlib",
];

/// Список прикладных пользовательских программ Wunix, устанавливаемых через WPM
pub const APP_PACKAGES: &[&str] = &[
    "archiveutils",
    "curl",
    "gzip",
    "nano",
    "openssh",
    "tar",
    "wc",
    "zip",
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

pub fn mount_root_partition(disk_path: &[u8], root_part_num: u8, fstype: &[u8]) -> bool {
    let _ = mkdir(b"/mnt\0", 0o755);

    let mut part_path = [0u8; 64];
    let mut len = 0;
    while len < disk_path.len() && disk_path[len] != 0 && len < 60 {
        part_path[len] = disk_path[len];
        len += 1;
    }
    part_path[len] = b'0' + root_part_num;
    part_path[len + 1] = 0;

    if mount(&part_path[..len + 2], b"/mnt\0", fstype, 0, b"\0") >= 0 {
        return true;
    }

    // Если раздел не зарегистрирован, пробуем смонтировать сам диск
    mount(disk_path, b"/mnt\0", fstype, 0, b"\0") >= 0
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
        copy_file(b"/boot/limine-bios.sys\0", b"/mnt_boot/boot/limine-bios.sys\0", 0o644);

        let mut conf_buf = [0u8; 512];
        let mut cpos = 0;
        cpos = ap(&mut conf_buf, cpos, b"timeout: 0\n\n/Wunix OS\n    protocol: limine\n    kernel_path: boot():/boot/wunix-kernel\n    cmdline: root=");
        cpos = ap(&mut conf_buf, cpos, root_dev_str.as_bytes());
        cpos = ap(&mut conf_buf, cpos, b" lang=ru mode=disk\n");
        let limine_conf_content = &conf_buf[..cpos];
        write_file(b"/mnt_boot/boot/limine.conf\0", limine_conf_content, 0o644);

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

/// Выполняет команду пакетного менеджера (wsp или wpm) через синхронный execve
fn run_command(cmd: &[u8], args: &[&[u8]]) -> bool {
    let log_fd = open(
        b"/tmp/winstall.log\0",
        (O_WRONLY | O_CREAT | O_APPEND) as u64,
        0o644,
    );
    let saved_stdout = libwunix::dup(1);
    let saved_stderr = libwunix::dup(2);

    if log_fd >= 0 {
        let _ = libwunix::dup2(log_fd as u64, 1);
        let _ = libwunix::dup2(log_fd as u64, 2);
        let _ = close(log_fd as u64);
    }

    let mut argv: [*const u8; 16] = [core::ptr::null(); 16];
    argv[0] = cmd.as_ptr();
    for (i, &arg) in args.iter().enumerate().take(14) {
        argv[i + 1] = arg.as_ptr();
    }
    argv[args.len() + 1] = core::ptr::null();

    let env_path = b"PATH=/bin\0";
    let envp: [*const u8; 2] = [env_path.as_ptr(), core::ptr::null()];

    let res = libwunix::execve(cmd, argv.as_ptr() as u64, envp.as_ptr() as u64);

    if saved_stdout >= 0 {
        let _ = libwunix::dup2(saved_stdout as u64, 1);
        let _ = close(saved_stdout as u64);
    }
    if saved_stderr >= 0 {
        let _ = libwunix::dup2(saved_stderr as u64, 2);
        let _ = close(saved_stderr as u64);
    }

    res == 0
}

fn copy_repo_metadata() {
    // Копируем системные пакеты WSP в кэш целевой системы
    const WSP_CACHE_FILES: &[&[u8]] = &[
        b"wunix-kernel-1.0.0.wp\0",
        b"wunix-kernel-1.0.1.wp\0",
        b"wunix-kernel-1.0.2.wp\0",
        b"libwunix-1.0.0.wp\0",
        b"libpackwunix-1.0.0.wp\0",
        b"libcwunix-1.0.0.wp\0",
        b"wunix-base-1.0.0.wp\0",
        b"wunix-base-1.0.1.wp\0",
        b"wunix-base-1.0.2.wp\0",
        b"coreutils-1.0.0.wp\0",
        b"coreutils-1.0.1.wp\0",
        b"coreutils-1.0.2.wp\0",
        b"netutils-1.0.0.wp\0",
        b"netutils-1.0.1.wp\0",
        b"netutils-1.0.2.wp\0",
        b"sysutils-1.0.0.wp\0",
        b"sysutils-1.0.1.wp\0",
        b"sysutils-1.0.2.wp\0",
        b"authutils-1.0.0.wp\0",
        b"authutils-1.0.1.wp\0",
        b"authutils-1.0.2.wp\0",
        b"wspd-1.0.0.wp\0",
        b"wspd-1.0.1.wp\0",
        b"wspd-1.0.2.wp\0",
        b"openssl-3.4.0.wp\0",
        b"ncurses-6.4.0.wp\0",
        b"zlib-1.3.1.wp\0",
    ];

    copy_file(b"/var/lib/wsp/repo/system/x86_64/packages.json\0", b"/mnt/var/lib/wsp/repo/system/x86_64/packages.json\0", 0o644);
    copy_file(b"/var/cache/wsp/packages.json\0", b"/mnt/var/cache/wsp/packages.json\0", 0o644);

    for &f in WSP_CACHE_FILES {
        let mut src = [0u8; 128];
        let mut dst = [0u8; 128];
        let mut spos = ap(&mut src, 0, b"/var/cache/wsp/");
        spos = ap(&mut src, spos, f);
        src[spos] = 0;

        let mut dpos = ap(&mut dst, 0, b"/mnt/var/cache/wsp/");
        dpos = ap(&mut dst, dpos, f);
        dst[dpos] = 0;

        copy_file(&src[..spos + 1], &dst[..dpos + 1], 0o644);
    }

    // Копируем прикладные пакеты WPM в кэш целевой системы
    const WPM_CACHE_FILES: &[&[u8]] = &[
        b"archiveutils-1.0.0.wp\0",
        b"curl-1.0.0.wp\0",
        b"git-2.43.0.wp\0",
        b"gzip-1.0.0.wp\0",
        b"nano-7.2.0.wp\0",
        b"openssh-9.8.0.wp\0",
        b"tar-1.0.0.wp\0",
        b"tinycc-0.9.27.wp\0",
        b"wc-1.0.0.wp\0",
        b"wc-1.0.1.wp\0",
        b"zip-1.0.0.wp\0",
    ];

    copy_file(b"/var/lib/wpm/repo/apps/x86_64/packages.json\0", b"/mnt/var/lib/wpm/repo/apps/x86_64/packages.json\0", 0o644);
    copy_file(b"/var/cache/wpm/packages.json\0", b"/mnt/var/cache/wpm/packages.json\0", 0o644);

    for &f in WPM_CACHE_FILES {
        let mut src = [0u8; 128];
        let mut dst = [0u8; 128];
        let mut spos = ap(&mut src, 0, b"/var/cache/wpm/");
        spos = ap(&mut src, spos, f);
        src[spos] = 0;

        let mut dpos = ap(&mut dst, 0, b"/mnt/var/cache/wpm/");
        dpos = ap(&mut dst, dpos, f);
        dst[dpos] = 0;

        copy_file(&src[..spos + 1], &dst[..dpos + 1], 0o644);
    }
}

pub fn total_install_items() -> usize {
    SYSTEM_PACKAGES.len() + APP_PACKAGES.len() + INSTALL_BOOT_FILES.len() + INSTALL_ETC_FILES.len() + 10
}

/// Выполняет установку системы Wunix на смонтированный диск
pub fn install_base<F>(
    disk_path: &[u8],
    root_part_num: u8,
    boot_part_num: u8,
    fs_type: &[u8],
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
    if !mount_root_partition(disk_path, root_part_num, fs_type) {
        return false;
    }
    done += 1;

    progress(5, "Создание структуры каталогов...", done, total);
    for d in &[
        b"/mnt/bin\0".as_ref(), b"/mnt/etc\0", b"/mnt/tmp\0",
        b"/mnt/root\0", b"/mnt/home\0", b"/mnt/mnt\0",
        b"/mnt/boot\0", b"/mnt/dev\0",  b"/mnt/proc\0", b"/mnt/sys\0",
        b"/mnt/lib\0", b"/mnt/lib64\0",
        b"/mnt/etc/ssl\0", b"/mnt/etc/ssl/certs\0",
        b"/mnt/etc/ssh\0", b"/mnt/etc/services\0", b"/mnt/etc/services/enabled\0",
        b"/mnt/etc/wsp\0", b"/mnt/etc/wpm\0",
        b"/mnt/var\0", b"/mnt/var/run\0", b"/mnt/var/empty\0", b"/mnt/var/log\0", b"/mnt/var/log/services\0",
        b"/mnt/var/lib/wsp\0", b"/mnt/var/lib/wpm\0",
        b"/mnt/var/lib/wsp/repo\0", b"/mnt/var/lib/wsp/repo/system\0", b"/mnt/var/lib/wsp/repo/system/x86_64\0",
        b"/mnt/var/lib/wpm/repo\0", b"/mnt/var/lib/wpm/repo/apps\0", b"/mnt/var/lib/wpm/repo/apps/x86_64\0",
        b"/mnt/var/cache/wsp\0", b"/mnt/var/cache/wpm\0",
        b"/mnt/run\0", b"/mnt/run/services\0",
        b"/mnt/usr\0", b"/mnt/usr/bin\0", b"/mnt/usr/lib\0", b"/mnt/usr/libexec\0",
        b"/mnt/usr/include\0", b"/mnt/usr/share\0",
    ] {
        ensure_dir(d);
    }
    let _ = libwunix::chmod(b"/mnt/tmp\0", 0o1777);
    let _ = libwunix::chmod(b"/mnt/root\0", 0o700);
    let _ = libwunix::chmod(b"/mnt/var/empty\0", 0o755);
    let _ = libwunix::chmod(b"/mnt/var/cache/wpm\0", 0o777);
    let _ = libwunix::chmod(b"/mnt/var/lib/wpm\0", 0o777);
    let _ = libwunix::chmod(b"/mnt/var/lib/wpm/repo\0", 0o777);
    let _ = libwunix::chmod(b"/mnt/var/lib/wpm/repo/apps\0", 0o777);
    let _ = libwunix::chmod(b"/mnt/var/lib/wpm/repo/apps/x86_64\0", 0o777);
    copy_repo_metadata();
    done += 1;

    // 1. Установка компонентов ядра и базовой системы через WSP
    for &pkg in SYSTEM_PACKAGES {
        let pct = (10 + (done * 45 / total)).min(55) as u8;
        progress(pct, pkg, done, total);

        let mut null_pkg = [0u8; 32];
        let b = pkg.as_bytes();
        let l = b.len().min(31);
        null_pkg[..l].copy_from_slice(&b[..l]);
        null_pkg[l] = 0;

        let _ = run_command(
            b"/bin/wsp\0",
            &[b"--root\0", b"/mnt\0", b"install\0", b"-y\0", b"--reinstall\0", &null_pkg[..l + 1]],
        );
        done += 1;
    }

    // 2. Установка пользовательских прикладных программ через WPM
    for &app in APP_PACKAGES {
        let pct = (55 + (done * 20 / total)).min(75) as u8;
        progress(pct, app, done, total);

        let mut null_app = [0u8; 32];
        let b = app.as_bytes();
        let l = b.len().min(31);
        null_app[..l].copy_from_slice(&b[..l]);
        null_app[l] = 0;

        let _ = run_command(
            b"/bin/wpm\0",
            &[b"--root\0", b"/mnt\0", b"install\0", b"-y\0", b"--reinstall\0", &null_app[..l + 1]],
        );
        done += 1;
    }

    // Настраиваем системные симлинки совместимости и бинарники сетевых служб
    let _ = symlink(b"/bin/wsh\0", b"/mnt/bin/sh\0");
    let _ = symlink(b"/lib/libcwunix.so\0", b"/mnt/lib/ld-musl-x86_64.so.1\0");
    let _ = symlink(b"/lib/libcwunix.so\0", b"/mnt/lib64/ld-musl-x86_64.so.1\0");
    let _ = symlink(b"/lib/libcwunix.so\0", b"/mnt/lib/ld-wunix.so.1\0");
    let _ = symlink(b"/lib/libcwunix.so\0", b"/mnt/lib64/ld-wunix.so.1\0");
    copy_file(b"/bin/sshd\0", b"/mnt/bin/sshd\0", 0o755);
    copy_file(b"/bin/ssh\0", b"/mnt/bin/ssh\0", 0o755);
    copy_file(b"/bin/ssh-keygen\0", b"/mnt/bin/ssh-keygen\0", 0o755);
    copy_file(b"/bin/scp\0", b"/mnt/bin/scp\0", 0o755);
    copy_file(b"/bin/sshd-session\0", b"/mnt/bin/sshd-session\0", 0o755);
    copy_file(b"/bin/sshd-session\0", b"/mnt/usr/libexec/sshd-session\0", 0o755);
    copy_file(b"/bin/sftp-server\0", b"/mnt/bin/sftp-server\0", 0o755);
    copy_file(b"/bin/sftp-server\0", b"/mnt/usr/libexec/sftp-server\0", 0o755);

    // 3. Копирование файлов ядра и загрузчика (/boot)
    progress(76, "Установка ядра и загрузчика (/boot)...", done, total);
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

    // Устанавливаем строгие права 0600 на закрытые серверные ключи OpenSSH
    let _ = libwunix::chmod(b"/mnt/etc/ssh/ssh_host_ed25519_key\0", 0o600);
    let _ = libwunix::chmod(b"/mnt/etc/ssh/ssh_host_rsa_key\0", 0o600);

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
command = /bin/sshd\n\
autostart = yes\n\
restart = always\n\
user = root\n\
directory = /\n";

    const WSPD_CFG: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb8 \xd0\xbe\xd0\xb1\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xb8\xd0\xb9 WSP\n\
description = \xd0\xa4\xd0\xbe\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xb0\xd1\x8f \xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0 \xd0\xbe\xd0\xb1\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xb8\xd0\xb9 \xd0\xb1\xd0\xb5\xd0\xb7\xd0\xbe\xd0\xbf\xd0\xb0\xd1\x81\xd0\xbd\xd0\xbe\xd1\x81\xd1\x82\xd0\xb8 WSP\n\
command = /bin/wspd -1\n\
autostart = yes\n\
restart = on-failure\n\
user = root\n\
directory = /\n";

    write_file(b"/mnt/etc/services/timesyncd.conf\0", TIMESYNCD_CFG, 0o644);
    write_file(b"/mnt/etc/services/network.conf\0", NETWORK_CFG, 0o644);
    write_file(b"/mnt/etc/services/sshd.conf\0", SSH_CFG, 0o644);
    write_file(b"/mnt/etc/services/wspd.conf\0", WSPD_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/timesyncd.conf\0", TIMESYNCD_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/network.conf\0", NETWORK_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/sshd.conf\0", SSH_CFG, 0o644);
    write_file(b"/mnt/etc/services/enabled/wspd.conf\0", WSPD_CFG, 0o644);

    // Конфигурационные файлы пакетных менеджеров WSP и WPM
    const WSP_DAEMON_CFG: &str = "# Конфигурация фонового демона WSP (wspd)\n[daemon]\nenabled = true\ncheck_interval = 3600\ncheck_on_boot = true\nblock_on_critical = true\nlog_file = \"/var/log/services/wspd.log\"\n";
    const WSP_CONF: &str = "# Системный менеджер пакетов WSP\n[system]\ncache_dir = \"/var/cache/wsp\"\nbackup_kernel = true\nblock_user_packages_on_critical = true\n";
    const WPM_CONF: &str = "# Пользовательский менеджер пакетов WPM\n[general]\ncache_dir = \"/var/cache/wpm\"\nverify_checksum = true\nblock_if_wsp_outdated = true\n";
    const WPM_SOURCES_CFG: &str = "# Источники репозиториев WPM\n[apps]\nServer = http://wunix.rusos.site/repo/apps/x86_64\n";

    write_file(b"/mnt/etc/wsp/daemon.conf\0", WSP_DAEMON_CFG.as_bytes(), 0o644);
    write_file(b"/mnt/etc/wsp/wsp.conf\0", WSP_CONF.as_bytes(), 0o644);
    write_file(b"/mnt/etc/wpm/wpm.conf\0", WPM_CONF.as_bytes(), 0o644);
    write_file(b"/mnt/etc/wpm/sources.conf\0", WPM_SOURCES_CFG.as_bytes(), 0o644);

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
        pos = ap(&mut pb, pos, b"nobody:x:65534:65534:Nobody:/:/bin/false\n");
        if !admin_user.is_empty() {
            pos = ap(&mut pb, pos, admin_user.as_bytes());
            pos = ap(&mut pb, pos, b":x:1000:1000:Admin:/home/");
            pos = ap(&mut pb, pos, admin_user.as_bytes());
            pos = ap(&mut pb, pos, b":/bin/wsh\n");
        }
        write_file(b"/mnt/etc/passwd\0", &pb[..pos], 0o644);
        write_file(b"/etc/passwd\0", &pb[..pos], 0o644);

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
        write_file(b"/etc/shadow\0", &sb[..pos], 0o600);

        // /etc/group
        let mut gb = [0u8; 256];
        let mut pos = ap(&mut gb, 0, b"root:x:0:root\nwheel:x:10:root");
        if !admin_user.is_empty() {
            pos = ap(&mut gb, pos, b",");
            pos = ap(&mut gb, pos, admin_user.as_bytes());
        }
        pos = ap(&mut gb, pos, b"\nnogroup:x:65534:\n");
        write_file(b"/mnt/etc/group\0", &gb[..pos], 0o644);
        write_file(b"/etc/group\0", &gb[..pos], 0o644);
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

        let mut hd_live = [0u8; 64];
        let mut pos_l = ap(&mut hd_live, 0, b"/home/");
        pos_l = ap(&mut hd_live, pos_l, admin_user.as_bytes());
        hd_live[pos_l] = 0;
        ensure_dir(&hd_live[..pos_l + 1]);
        let _ = libwunix::chown(&hd_live[..pos_l + 1], 1000, 1000);
        let _ = libwunix::chmod(&hd_live[..pos_l + 1], 0o750);
    }
    done += 1;

    // 9. Маркер установленной системы
    progress(90, "Фиксация флага установки...", done, total);
    write_file(
        b"/mnt/etc/installed\0",
        b"WUNIX_INSTALLED=1\nINSTALL_STAGE=11\nRELEASE=1.0\n",
        0o644,
    );
    write_file(
        b"/etc/installed\0",
        b"WUNIX_INSTALLED=1\nINSTALL_STAGE=11\nRELEASE=1.0\n",
        0o644,
    );
    done += 1;

    // Гарантируем, что утилита установки и её документация не остаются в установленной системе
    let _ = unlink(b"/mnt/bin/winstall\0");
    let _ = unlink(b"/mnt/usr/share/man/man8/winstall.8\0");
    let _ = unlink(b"/mnt/usr/share/man/ru/man8/winstall.8\0");

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
