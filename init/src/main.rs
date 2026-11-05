//! Системный супервизор и диспетчер инициализации Wunix (PID 1).
//!
//! Отвечает за:
//! 1. Первичное монтирование системных структур VFS.
//! 2. Инициализацию каталогов супервизора и запуск автостартующих служб (/etc/services/enabled).
//! 3. Анализ командной строки ядра /proc/cmdline (диск, live, инсталлятор).
//! 4. Супервизию пользовательской консоли (wsh / login) и очистку процессов-сирот.

#![no_std]
#![no_main]

mod disk;
mod services;

use disk::check_installed;
use libwunix::{
    chdir, close, execve, open, pread64, t, write, O_RDONLY, STDERR_FILENO, STDOUT_FILENO,
};
use services::init_services;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // 1. Инициализируем супервизор и системные службы Wunix (/etc/services/enabled)
    init_services();

    // 2. Проверяем параметры командной строки ядра /proc/cmdline
    let mut cmdline_buf = [0u8; 256];
    let mut is_disk_mode = false;
    let mut is_install_mode = false;
    let mut is_live_mode = false;

    let cmd_fd = open(b"/proc/cmdline\0", O_RDONLY as u64, 0);
    if cmd_fd >= 0 {
        let n = pread64(cmd_fd as u64, &mut cmdline_buf, 0);
        close(cmd_fd as u64);
        if n > 0 {
            let cmd_str = core::str::from_utf8(&cmdline_buf[..n as usize]).unwrap_or("");
            if cmd_str.contains("root=/dev/") || cmd_str.contains("mode=disk") {
                is_disk_mode = true;
            } else if cmd_str.contains("mode=install") {
                is_install_mode = true;
            } else if cmd_str.contains("mode=live") {
                is_live_mode = true;
            }
        }
    }

    // 3. Маршрутизация целевого режима: при загрузке с диска запускаем экран входа (login)
    if is_disk_mode && check_installed() {
        let login_argv = [b"/bin/login\0".as_ptr(), core::ptr::null()];
        loop {
            let _ = libwunix::setgid(0);
            let _ = libwunix::setuid(0);
            let _ = chdir(b"/\0");
            let ret = execve(b"/bin/login\0", login_argv.as_ptr() as u64, 0);
            if ret < 0 {
                let wsh_argv = [b"/bin/wsh\0".as_ptr(), core::ptr::null()];
                let _ = execve(b"/bin/wsh\0", wsh_argv.as_ptr() as u64, 0);
                let req = [1u64, 0u64];
                libwunix::nanosleep(req.as_ptr() as *const u8, core::ptr::null_mut());
            }
            libwunix::yield_now();
        }
    } else if is_install_mode {
        let winstall_argv = [b"/bin/winstall\0".as_ptr(), core::ptr::null()];
        let _ = execve(b"/bin/winstall\0", winstall_argv.as_ptr() as u64, 0);
    } else if is_live_mode {
        write(
            STDOUT_FILENO,
            t(
                "\x1b[1;36mДобро пожаловать в Wunix OS (Live-сессия)!\x1b[0m\n\n",
                "\x1b[1;36mWelcome to Wunix OS (Live session)!\x1b[0m\n\n",
            )
            .as_bytes(),
        );
        let _ = chdir(b"/root\0");
        let wsh_argv = [b"/bin/wsh\0".as_ptr(), core::ptr::null()];
        let ret = execve(b"/bin/wsh\0", wsh_argv.as_ptr() as u64, 0);
        if ret < 0 {
            let _ = write(
                STDOUT_FILENO,
                b"[init] \x1b[1;31m\xd0\x9e\xd1\x88\xd0\xb8\xd0\xb1\xd0\xba\xd0\xb0 \xd0\xb7\xd0\xb0\xd0\xbf\xd1\x83\xd1\x81\xd0\xba\xd0\xb0 /bin/wsh\x1b[0m\n",
            );
        }
    } else {
        // По умолчанию на установочном ISO запускаем TUI загрузочное меню (winstall)
        let winstall_argv = [b"/bin/winstall\0".as_ptr(), core::ptr::null()];
        let _ = execve(b"/bin/winstall\0", winstall_argv.as_ptr() as u64, 0);
    }

    // 4. Если основной процесс завершился, супервизор удерживает систему через login / wsh
    let login_argv = [b"/bin/login\0".as_ptr(), core::ptr::null()];
    loop {
        let _ = libwunix::setgid(0);
        let _ = libwunix::setuid(0);
        let _ = chdir(b"/\0");
        let ret = execve(b"/bin/login\0", login_argv.as_ptr() as u64, 0);
        if ret < 0 {
            let wsh_argv = [b"/bin/wsh\0".as_ptr(), core::ptr::null()];
            let _ = execve(b"/bin/wsh\0", wsh_argv.as_ptr() as u64, 0);
            let req = [1u64, 0u64];
            libwunix::nanosleep(req.as_ptr() as *const u8, core::ptr::null_mut());
        }
        libwunix::yield_now();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in init (PID 1)\n");
    libwunix::exit(1);
}
