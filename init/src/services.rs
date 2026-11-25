//! Инициализация и автозапуск системных служб Wunix при старте ядра.

use libwunix::{
    chmod, close, execve, getdents64, mkdir, open, pread64, pwrite64, symlink, unlink, write,
    O_CREAT, O_EXCL, O_RDONLY, O_WRONLY, STDOUT_FILENO, t,
};

const TIMESYNCD_DEFAULT: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd1\x81\xd0\xb8\xd0\xbd\xd1\x85\xd1\x80\xd0\xbe\xd0\xbd\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd0\xb8 \xd0\xb2\xd1\x80\xd0\xb5\xd0\xbc\xd0\xb5\xd0\xbd\xd0\xb8 Wunix NTP\n\
description = \xd0\xa1\xd0\xb8\xd0\xbd\xd1\x85\xd1\x80\xd0\xbe\xd0\xbd\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd1\x8f \xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xbd\xd1\x8b\xd1\x85 \xd1\x87\xd0\xb0\xd1\x81\xd0\xbe\xd0\xb2 \xd0\xbf\xd0\xbe \xd0\xbf\xd1\x80\xd0\xbe\xd1\x82\xd0\xbe\xd0\xba\xd0\xbe\xd0\xbb\xd1\x83 NTP\n\
command = /bin/timesyncd -1\n\
autostart = yes\n\
restart = on-failure\n\
user = root\n\
directory = /\n";

const NETWORK_DEFAULT: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd0\xbd\xd0\xb0\xd1\x81\xd1\x82\xd1\x80\xd0\xbe\xd0\xb9\xd0\xba\xd0\xb8 \xd1\x81\xd0\xb5\xd1\x82\xd0\xb8\n\
description = \xd0\x9f\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0 \xd0\xb8 \xd0\xb8\xd0\xbd\xd0\xb8\xd1\x86\xd0\xb8\xd0\xb0\xd0\xbb\xd0\xb8\xd0\xb7\xd0\xb0\xd1\x86\xd0\xb8\xd1\x8f \xd1\x81\xd0\xb5\xd1\x82\xd0\xb5\xd0\xb2\xd1\x8b\xd1\x85 \xd0\xb8\xd0\xbd\xd1\x82\xd0\xb5\xd1\x80\xd1\x84\xd0\xb5\xd0\xb9\xd1\x81\xd0\xbe\xd0\xb2\n\
command = /bin/ip addr\n\
autostart = yes\n\
restart = no\n\
user = root\n\
directory = /\n";

const SSH_DEFAULT: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 OpenSSH\n\
description = \xd0\xa1\xd0\xb5\xd1\x82\xd0\xb5\xd0\xb2\xd0\xbe\xd0\xb9 \xd1\x81\xd0\xb5\xd1\x80\xd0\xb2\xd0\xb5\xd1\x80 \xd1\x83\xd0\xb4\xd0\xb0\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xbd\xd0\xbe\xd0\xb3\xd0\xbe \xd0\xb4\xd0\xbe\xd1\x81\xd1\x82\xd1\x83\xd0\xbf\xd0\xb0 SSH\n\
command = /bin/sshd\n\
autostart = yes\n\
restart = always\n\
user = root\n\
directory = /\n";

const WSPD_DEFAULT: &[u8] = b"# \xd0\xa1\xd0\xbb\xd1\x83\xd0\xb6\xd0\xb1\xd0\xb0 \xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb8 \xd0\xbe\xd0\xb1\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xb8\xd0\xb9 WSP\n\
description = \xd0\xa4\xd0\xbe\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xb0\xd1\x8f \xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0 \xd0\xbe\xd0\xb1\xd0\xbd\xd0\xbe\xd0\xb2\xd0\xbb\xd0\xb5\xd0\xbd\xd0\xb8\xd0\xb9 \xd0\xb1\xd0\xb5\xd0\xb7\xd0\xbe\xd0\xbf\xd0\xb0\xd1\x81\xd0\xbd\xd0\xbe\xd1\x81\xd1\x82\xd0\xb8 WSP\n\
command = /bin/wspd -1\n\
autostart = yes\n\
restart = on-failure\n\
user = root\n\
directory = /\n";

fn print_str(s: &str) {
    let _ = write(STDOUT_FILENO, s.as_bytes());
}

fn print_usize(mut n: usize) {
    let mut buf = [0u8; 16];
    if n == 0 {
        let _ = write(STDOUT_FILENO, b"0");
        return;
    }
    let mut idx = 0;
    while n > 0 && idx < buf.len() {
        buf[idx] = b'0' + (n % 10) as u8;
        n /= 10;
        idx += 1;
    }
    while idx > 0 {
        idx -= 1;
        let _ = write(STDOUT_FILENO, &buf[idx..idx + 1]);
    }
}

pub fn init_services() {
    // 1. Создаем обязательные каталоги супервизора
    print_str(t(
        "Подготовка каталогов /run, /var/log, /etc/services.\n",
        "Preparing /run, /var/log, /etc/services.\n",
    ));
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/services\0", 0o755);
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/run\0", 0o755);
    let _ = mkdir(b"/var/log\0", 0o755);
    let _ = mkdir(b"/var/log/services\0", 0o755);
    let _ = mkdir(b"/var/empty\0", 0o755);
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/ssh\0", 0o755);
    let _ = mkdir(b"/etc/services\0", 0o755);
    let _ = mkdir(b"/etc/services/enabled\0", 0o755);

    let _ = mkdir(b"/usr\0", 0o755);
    let _ = mkdir(b"/usr/libexec\0", 0o755);
    copy_if_missing(b"/bin/sshd-session\0", b"/usr/libexec/sshd-session\0", 0o755);
    copy_if_missing(b"/bin/sftp-server\0", b"/usr/libexec/sftp-server\0", 0o755);

    // Очищаем runtime-состояния и PID-файлы от предыдущих загрузок/сессий
    let cleaned = clean_runtime_dir();
    if cleaned > 0 {
        print_str(t(
            "Очистка /run/services: удалено файлов: ",
            "Cleaned /run/services stale files: ",
        ));
        print_usize(cleaned);
        print_str(".\n");
    }

    // Устанавливаем строгие права 0600 на закрытые серверные ключи OpenSSH
    let _ = chmod(b"/etc/ssh/ssh_host_ed25519_key\0", 0o600);
    let _ = chmod(b"/etc/ssh/ssh_host_rsa_key\0", 0o600);

    // 2. Создаем базовые файлы конфигураций, если отсутствуют
    create_service_if_missing(b"/etc/services/timesyncd.conf\0", TIMESYNCD_DEFAULT);
    create_service_if_missing(b"/etc/services/network.conf\0", NETWORK_DEFAULT);
    create_service_if_missing(b"/etc/services/sshd.conf\0", SSH_DEFAULT);
    create_service_if_missing(b"/etc/services/wspd.conf\0", WSPD_DEFAULT);

    // 3. Включаем автозапуск по умолчанию для базовых служб
    let _ = symlink(
        b"/etc/services/timesyncd.conf\0",
        b"/etc/services/enabled/timesyncd.conf\0",
    );
    let _ = symlink(
        b"/etc/services/network.conf\0",
        b"/etc/services/enabled/network.conf\0",
    );
    let _ = symlink(
        b"/etc/services/sshd.conf\0",
        b"/etc/services/enabled/sshd.conf\0",
    );
    let _ = symlink(
        b"/etc/services/wspd.conf\0",
        b"/etc/services/enabled/wspd.conf\0",
    );

    // 4. Запускаем ключевые службы в классическом Unix/BSD стиле
    print_str(t(
        "Запуск системных служб (/etc/services/enabled):\n",
        "Starting system services (/etc/services/enabled):\n",
    ));

    const BOOT_SERVICES: [(&str, &str, &str); 4] = [
        ("network", "сетевые интерфейсы", "network interfaces"),
        ("timesyncd", "синхронизация времени NTP", "NTP clock sync"),
        ("sshd", "сервер удалённого доступа SSH", "OpenSSH daemon"),
        ("wspd", "проверка обновлений WSP", "WSP update check"),
    ];

    let mut ok_count = 0usize;
    for (svc, desc_ru, desc_en) in BOOT_SERVICES {
        print_str("  ");
        print_str(svc);
        print_str(" (");
        print_str(t(desc_ru, desc_en));
        print_str("): ");
        if start_one_service(svc) {
            ok_count += 1;
        }
    }

    print_str(t(
        "Системные службы запущены (",
        "System services started (",
    ));
    print_usize(ok_count);
    print_str("/");
    print_usize(BOOT_SERVICES.len());
    print_str(").\n");
}

fn create_service_if_missing(path: &[u8], content: &[u8]) {
    let fd = open(path, (O_WRONLY | O_CREAT | O_EXCL) as u64, 0o644);
    if fd >= 0 {
        write(fd as u64, content);
        close(fd as u64);
    }
}

fn copy_if_missing(src: &[u8], dst: &[u8], mode: u64) {
    let dfd = open(dst, (O_WRONLY | O_CREAT | O_EXCL) as u64, mode);
    if dfd < 0 {
        return;
    }
    let sfd = open(src, O_RDONLY as u64, 0);
    if sfd < 0 {
        close(dfd as u64);
        return;
    }
    let mut buf = [0u8; 8192];
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

fn start_one_service(clean_name: &str) -> bool {
    let mut null_name = [0u8; 64];
    let b = clean_name.as_bytes();
    if b.len() >= null_name.len() {
        return false;
    }
    null_name[..b.len()].copy_from_slice(b);
    null_name[b.len()] = 0;

    let sys_bin = b"/bin/system\0";
    let start_arg = b"start\0";

    let argv_ptrs: [*const u8; 4] = [
        sys_bin.as_ptr(),
        start_arg.as_ptr(),
        null_name.as_ptr(),
        core::ptr::null(),
    ];

    execve(sys_bin, argv_ptrs.as_ptr() as u64, 0) == 0
}

fn clean_runtime_dir() -> usize {
    let mut removed = 0usize;
    let fd = open(b"/run/services\0", O_RDONLY as u64, 0);
    if fd < 0 {
        return 0;
    }
    let mut buf = [0u8; 1024];
    loop {
        let n = getdents64(fd as u64, &mut buf);
        if n <= 0 {
            break;
        }
        let total = (n as usize).min(buf.len());
        let mut pos = 0;
        while pos + 19 <= total {
            let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
            if reclen == 0 || pos + reclen > total {
                break;
            }
            let name_start = pos + 19;
            let mut name_end = name_start;
            let max_end = pos + reclen;
            while name_end < max_end && buf[name_end] != 0 {
                name_end += 1;
            }
            if name_end > name_start {
                let name = &buf[name_start..name_end];
                if name != b"." && name != b".." {
                    let mut path = [0u8; 256];
                    let prefix = b"/run/services/";
                    if prefix.len() + name.len() < path.len() - 1 {
                        path[..prefix.len()].copy_from_slice(prefix);
                        path[prefix.len()..prefix.len() + name.len()].copy_from_slice(name);
                        path[prefix.len() + name.len()] = 0;
                        if unlink(&path[..prefix.len() + name.len() + 1]) == 0 {
                            removed += 1;
                        }
                    }
                }
            }
            pos += reclen;
        }
    }
    let _ = close(fd as u64);
    removed
}
