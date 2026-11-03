//! Утилита управления системными службами Wunix (`system`).
//!
//! Обеспечивает простой, легковесный и полностью функциональный интерфейс:
//! start, stop, restart, enable, disable, status, list, edit, create, logs.

#![no_std]
#![no_main]

extern crate alloc;

mod args;
mod config;
mod editor;
mod logs;
mod service;
mod status;

use alloc::vec::Vec;
use args::{SystemAction, SystemArgs};
use libwunix::{exit, println, str_utils, t};

const SYSTEM_HEAP_SIZE: usize = 1024 * 1024; // 1 МиБ кучи для парсинга конфигураций и строк

struct SimpleBumpAllocator {
    heap: core::cell::UnsafeCell<[u8; SYSTEM_HEAP_SIZE]>,
    offset: core::sync::atomic::AtomicUsize,
}

unsafe impl Sync for SimpleBumpAllocator {}

unsafe impl core::alloc::GlobalAlloc for SimpleBumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut cur = self.offset.load(core::sync::atomic::Ordering::Relaxed);
        loop {
            let aligned = (cur + align - 1) & !(align - 1);
            let next = aligned + size;
            if next > SYSTEM_HEAP_SIZE {
                return core::ptr::null_mut();
            }
            match self.offset.compare_exchange_weak(
                cur,
                next,
                core::sync::atomic::Ordering::SeqCst,
                core::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let ptr = self.heap.get() as *mut u8;
                    return unsafe { ptr.add(aligned) };
                }
                Err(actual) => cur = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static ALLOCATOR: SimpleBumpAllocator = SimpleBumpAllocator {
    heap: core::cell::UnsafeCell::new([0u8; SYSTEM_HEAP_SIZE]),
    offset: core::sync::atomic::AtomicUsize::new(0),
};

fn cstr_from_ptr(ptr: *const u8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let len = str_utils::strlen(ptr);
    core::str::from_utf8(unsafe { core::slice::from_raw_parts(ptr, len) }).unwrap_or("")
}

fn print_help() {
    println!(
        "{}",
        t(
            "Использование: system <КОМАНДА> [ПАРАМЕТРЫ]...\n\
             Система управления и супервизии служб операционной системы Wunix.\n\n\
             Управление службами:\n  \
               start <СЛУЖБА>       Запустить указанную службу\n  \
               stop <СЛУЖБА>        Остановить указанную службу\n  \
               restart <СЛУЖБА>     Перезапустить указанную службу\n  \
               enable <СЛУЖБА>      Включить автозапуск службы при загрузке ОС\n  \
               disable <СЛУЖБА>     Отключить автозапуск службы при загрузке ОС\n  \
               status [СЛУЖБА]      Отобразить статус службы или список всех служб\n  \
               list                 Вывести список всех установленных служб и их состояние\n  \
               edit <СЛУЖБА>        Открыть файл службы в редакторе nano\n  \
               create <СЛУЖБА>      Создать новую службу по шаблону и открыть в nano\n  \
               logs <СЛУЖБА>        Просмотреть журнал службы (/var/log/services/)\n\n\
             Параметры команды logs:\n  \
               -n, --lines <N>      Показать последние N строк (по умолчанию 20)\n  \
               --head <N>           Показать первые N строк\n  \
               -f, --follow         Отслеживать журнал в реальном времени\n\n\
             Общие параметры:\n  \
               -h, --help           Показать эту справку и выйти\n  \
               -V, --version        Показать версию утилиты\n\n\
             Конфигурации служб хранятся в /etc/services/*.conf.",
            "Usage: system <COMMAND> [OPTIONS]...\n\
             Wunix Service Management and Supervisor System.\n\n\
             Service Management Commands:\n  \
               start <SERVICE>      Start the specified service\n  \
               stop <SERVICE>       Stop the specified service\n  \
               restart <SERVICE>    Restart the specified service\n  \
               enable <SERVICE>     Enable service auto-start at boot\n  \
               disable <SERVICE>    Disable service auto-start at boot\n  \
               status [SERVICE]     Show status of service or list all services\n  \
               list                 List all installed services and their status\n  \
               edit <SERVICE>       Open service configuration in nano editor\n  \
               create <SERVICE>     Create a new service from template and open in nano\n  \
               logs <SERVICE>       View service logs (/var/log/services/)\n\n\
             Logs Options:\n  \
               -n, --lines <N>      Show last N lines (default 20)\n  \
               --head <N>           Show first N lines\n  \
               -f, --follow         Follow log output in real-time\n\n\
             General Options:\n  \
               -h, --help           Display this help and exit\n  \
               -V, --version        Show version information\n\n\
             Service configurations are located in /etc/services/*.conf."
        )
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 0 {
        let envp = unsafe { argv.add(argc as usize + 1) };
        libwunix::env::init(envp);
    }

    let mut args_vec: Vec<&str> = Vec::new();
    for i in 1..argc {
        args_vec.push(cstr_from_ptr(unsafe { *argv.add(i as usize) }));
    }

    let parsed = match SystemArgs::parse(&args_vec) {
        Ok(a) => a,
        Err(err) => {
            println!("{}", err);
            exit(1);
        }
    };

    match parsed.action {
        SystemAction::Start(name) => {
            match service::start_service(&name) {
                Ok(code) => {
                    if code == 0 {
                        println!(
                            "  {} '{}': {}",
                            t("Служба", "Service"),
                            name,
                            t("запущена", "started")
                        );
                        exit(0);
                    } else {
                        println!(
                            "  {} '{}': {} ({})",
                            t("Служба", "Service"),
                            name,
                            t("ошибка запуска", "start error"),
                            code
                        );
                        exit(1);
                    }
                }
                Err(e) => {
                    println!("  {}: {}", t("Ошибка запуска", "Failed to start"), e);
                    exit(1);
                }
            }
        }
        SystemAction::Stop(name) => {
            match service::stop_service(&name) {
                Ok(_) => {
                    println!(
                        "  {} '{}': {}",
                        t("Служба", "Service"),
                        name,
                        t("остановлена", "stopped")
                    );
                    exit(0);
                }
                Err(e) => {
                    println!("  {}: {}", t("Ошибка остановки", "Failed to stop"), e);
                    exit(1);
                }
            }
        }
        SystemAction::Restart(name) => {
            println!(
                "{} '{}'...",
                t("system: перезапуск службы", "system: restarting service"),
                name
            );
            match service::restart_service(&name) {
                Ok(code) => {
                    if code == 0 {
                        println!(
                            "[ \x1b[1;32mOK\x1b[0m ] {} '{}' {}.",
                            t("Служба", "Service"),
                            name,
                            t("успешно перезапущена", "restarted successfully")
                        );
                        exit(0);
                    } else {
                        println!(
                            "[ \x1b[1;31mERR\x1b[0m ] {} '{}' {} {}.",
                            t("Служба", "Service"),
                            name,
                            t("завершилась с кодом", "exited with code"),
                            code
                        );
                        exit(1);
                    }
                }
                Err(e) => {
                    println!("[ \x1b[1;31mERR\x1b[0m ] {}: {}", t("Ошибка перезапуска", "Failed to restart"), e);
                    exit(1);
                }
            }
        }
        SystemAction::Enable(name) => {
            match config::enable_service(&name) {
                Ok(_) => {
                    println!(
                        "[ \x1b[1;32mOK\x1b[0m ] {} '{}' {}.",
                        t("Автозапуск для службы", "Autostart for service"),
                        name,
                        t("включен", "enabled")
                    );
                    exit(0);
                }
                Err(e) => {
                    println!("[ \x1b[1;31mERR\x1b[0m ] {}: {}", t("Ошибка включения автозапуска", "Failed to enable autostart"), e);
                    exit(1);
                }
            }
        }
        SystemAction::Disable(name) => {
            match config::disable_service(&name) {
                Ok(_) => {
                    println!(
                        "[ \x1b[1;32mOK\x1b[0m ] {} '{}' {}.",
                        t("Автозапуск для службы", "Autostart for service"),
                        name,
                        t("отключен", "disabled")
                    );
                    exit(0);
                }
                Err(e) => {
                    println!("[ \x1b[1;31mERR\x1b[0m ] {}: {}", t("Ошибка отключения автозапуска", "Failed to disable autostart"), e);
                    exit(1);
                }
            }
        }
        SystemAction::Status(Some(name)) => {
            status::show_service_status(&name);
            exit(0);
        }
        SystemAction::Status(None) | SystemAction::List => {
            status::list_all_services();
            exit(0);
        }
        SystemAction::Edit(name) => {
            editor::edit_service(&name);
            exit(0);
        }
        SystemAction::Create(name) => {
            editor::create_service(&name);
            exit(0);
        }
        SystemAction::Logs {
            service,
            tail,
            head,
            follow,
        } => {
            logs::show_logs(&service, tail, head, follow);
            exit(0);
        }
        SystemAction::Help => {
            print_help();
            exit(0);
        }
        SystemAction::Version => {
            println!("wunix-system 1.0.0 (Wunix Service Supervisor)");
            exit(0);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("panic in system");
    exit(1);
}
