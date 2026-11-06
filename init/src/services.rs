//! Инициализация и автозапуск системных служб Wunix при старте ядра.

use libwunix::{
    chmod, close, execve, getdents64, mkdir, open, symlink, write, O_CREAT, O_EXCL, O_RDONLY, O_WRONLY,
    STDOUT_FILENO, t,
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
command = /bin/sshd -D\n\
autostart = yes\n\
restart = always\n\
user = root\n\
directory = /\n";

pub fn init_services() {
    // 1. Создаем обязательные каталоги супервизора
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/services\0", 0o755);
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/log\0", 0o755);
    let _ = mkdir(b"/var/log/services\0", 0o755);
    let _ = mkdir(b"/var/empty\0", 0o755);
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/ssh\0", 0o755);
    let _ = mkdir(b"/etc/services\0", 0o755);
    let _ = mkdir(b"/etc/services/enabled\0", 0o755);

    let _ = mkdir(b"/usr\0", 0o755);
    let _ = mkdir(b"/usr/libexec\0", 0o755);
    let _ = symlink(b"/bin/sshd-session\0", b"/usr/libexec/sshd-session\0");
    let _ = symlink(b"/bin/sftp-server\0", b"/usr/libexec/sftp-server\0");

    // Устанавливаем строгие права 0600 на закрытые серверные ключи OpenSSH
    let _ = chmod(b"/etc/ssh/ssh_host_ed25519_key\0", 0o600);
    let _ = chmod(b"/etc/ssh/ssh_host_rsa_key\0", 0o600);

    // 2. Создаем базовые файлы конфигураций, если отсутствуют
    create_service_if_missing(b"/etc/services/timesyncd.conf\0", TIMESYNCD_DEFAULT);
    create_service_if_missing(b"/etc/services/network.conf\0", NETWORK_DEFAULT);
    create_service_if_missing(b"/etc/services/sshd.conf\0", SSH_DEFAULT);

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

    // 4. Оповещаем пользователя о начале старта служб
    write(
        STDOUT_FILENO,
        t(
            "Инициализация служб...\n",
            "Initializing services...\n",
        )
        .as_bytes(),
    );

    // 5. Запускаем включенные службы через /bin/system start <служба>
    let fd = open(b"/etc/services/enabled\0", O_RDONLY as u64, 0);
    if fd >= 0 {
        let mut buf = [0u8; 2048];
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
                    let entry_slice = &buf[name_start..name_end];
                    if let Ok(entry_name) = core::str::from_utf8(entry_slice) {
                        if entry_name.ends_with(".conf") && !entry_name.starts_with('.') {
                            if let Some(clean) = entry_name.strip_suffix(".conf") {
                                start_one_service(clean);
                            }
                        }
                    }
                }

                pos += reclen;
            }
        }
        close(fd as u64);
    }

    write(
        STDOUT_FILENO,
        t(
            "Системные службы запущены.\n\n",
            "System services started.\n\n",
        )
        .as_bytes(),
    );
}

fn create_service_if_missing(path: &[u8], content: &[u8]) {
    let fd = open(path, (O_WRONLY | O_CREAT | O_EXCL) as u64, 0o644);
    if fd >= 0 {
        write(fd as u64, content);
        close(fd as u64);
    }
}

fn start_one_service(clean_name: &str) {
    let mut null_name = [0u8; 64];
    let b = clean_name.as_bytes();
    if b.len() >= null_name.len() {
        return;
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

    let _ = execve(sys_bin, argv_ptrs.as_ptr() as u64, 0);
}
