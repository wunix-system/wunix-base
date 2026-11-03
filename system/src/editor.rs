//! Интеграция с текстовым редактором nano для команд `system edit` и `system create`.

extern crate alloc;
use libwunix::{close, execve, open, println, t, write, O_CREAT, O_EXCL, O_WRONLY};
use crate::config::{create_template, ensure_dirs, file_exists, get_service_path, load_service};

pub fn edit_service(name: &str) {
    let path = get_service_path(name);
    if !file_exists(&path) {
        println!(
            "{} '{}' {}. {}",
            t("system: служба", "system: service"),
            name,
            t("не найдена в /etc/services", "not found in /etc/services"),
            t("Для создания новой службы используйте: system create <служба>", "To create a new service, use: system create <service>")
        );
        return;
    }

    launch_editor(&path);

    // Валидация синтаксиса после редактирования
    match load_service(name) {
        Ok(_) => {
            println!(
                "{} '{}' {}.",
                t("system: файл службы", "system: service file"),
                name,
                t("успешно сохранен и проверен", "successfully saved and verified")
            );
        }
        Err(e) => {
            println!(
                "{}: {}",
                t("system: внимание, в конфигурации обнаружены ошибки", "system: warning, configuration has errors"),
                e
            );
        }
    }
}

pub fn create_service(name: &str) {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let path = get_service_path(clean_name);

    ensure_dirs();

    if !file_exists(&path) {
        let template = create_template(clean_name);
        let mut null_path = [0u8; 256];
        let b = path.as_bytes();
        if b.len() >= null_path.len() {
            println!("{}", t("system: путь к службе слишком длинный", "system: service path too long"));
            return;
        }
        null_path[..b.len()].copy_from_slice(b);
        null_path[b.len()] = 0;

        let fd = open(&null_path[..b.len() + 1], (O_WRONLY | O_CREAT | O_EXCL) as u64, 0o644);
        if fd >= 0 {
            write(fd as u64, template.as_bytes());
            close(fd as u64);
        } else {
            println!("{}", t("system: ошибка создания файла службы", "system: failed to create service file"));
            return;
        }
    } else {
        println!(
            "{} '{}' {}.",
            t("system: служба", "system: service"),
            clean_name,
            t("уже существует, открываем для редактирования", "already exists, opening for editing")
        );
    }

    launch_editor(&path);

    // Проверяем валидность созданной службы
    match load_service(clean_name) {
        Ok(cfg) => {
            println!(
                "{} '{}' {}.",
                t("system: служба", "system: service"),
                clean_name,
                t("готова к использованию", "is ready for use")
            );
            if cfg.autostart {
                let _ = crate::config::enable_service(clean_name);
                println!(
                    "{} '{}' {}.",
                    t("system: автозапуск для службы", "system: autostart for service"),
                    clean_name,
                    t("включен", "enabled")
                );
            }
            println!(
                "{}: system start {}",
                t("Для запуска выполните", "To start now, run"),
                clean_name
            );
        }
        Err(e) => {
            println!(
                "{}: {}",
                t("system: внимание, в созданной конфигурации есть ошибки", "system: warning, created configuration has errors"),
                e
            );
        }
    }
}

fn launch_editor(target_path: &str) {
    let mut null_nano = [0u8; 32];
    let nb = b"/bin/nano";
    null_nano[..nb.len()].copy_from_slice(nb);
    null_nano[nb.len()] = 0;

    let mut null_target = [0u8; 256];
    let tb = target_path.as_bytes();
    if tb.len() >= null_target.len() {
        return;
    }
    null_target[..tb.len()].copy_from_slice(tb);
    null_target[tb.len()] = 0;

    let argv_ptrs: [*const u8; 3] = [
        null_nano.as_ptr(),
        null_target.as_ptr(),
        core::ptr::null(),
    ];

    let ret = execve(
        &null_nano[..nb.len() + 1],
        argv_ptrs.as_ptr() as u64,
        0,
    );

    if ret < 0 {
        println!(
            "{}",
            t(
                "system: не удалось запустить /bin/nano для редактирования.",
                "system: failed to execute /bin/nano for editing."
            )
        );
    }
}
