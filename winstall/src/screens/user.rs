// Создание первого пользователя-администратора (sudo / wheel) и задание имени хоста.

use crate::tui::{
    center_col, clear_screen, draw_centered, draw_footer, draw_header, draw_info,
    draw_input_field, draw_label, draw_logo, draw_password_field, draw_separator,
    hide_cursor, input_password, input_text, FG_RED, LOGO_ROW,
};

pub struct UserHostnameConfig {
    pub username: [u8; 64],
    pub username_len: usize,
    pub password: [u8; 128],
    pub password_len: usize,
    pub hostname: [u8; 64],
    pub hostname_len: usize,
}

impl Default for UserHostnameConfig {
    fn default() -> Self {
        let mut cfg = Self {
            username: [0; 64],
            username_len: 5,
            password: [0; 128],
            password_len: 0,
            hostname: [0; 64],
            hostname_len: 5,
        };
        cfg.username[..5].copy_from_slice(b"admin");
        cfg.hostname[..5].copy_from_slice(b"wunix");
        cfg
    }
}

pub fn screen_user_and_hostname() -> Option<UserHostnameConfig> {
    let mut cfg = UserHostnameConfig::default();
    let mut error_msg: Option<&str> = None;

    loop {
        hide_cursor();
        clear_screen();
        draw_header("  ЭТАП 5: УЧЕТНАЯ ЗАПИСЬ И ИМЯ ХОСТА  ");
        draw_logo(LOGO_ROW);

        draw_centered(9, "Создание первого администратора системы и задание сетевого имени:");

        let form_w = 64usize;
        let base_col = center_col(form_w);
        let label_col = base_col + 24;
        let field_col = base_col + 26;
        let field_w   = 34usize;

        draw_label(11, "Имя пользователя", label_col);
        draw_label(13, "Пароль", label_col);
        draw_label(15, "Повтор пароля", label_col);
        draw_label(17, "Имя хоста (Hostname)", label_col);

        draw_input_field(11, field_col, field_w, &cfg.username, cfg.username_len);
        draw_password_field(13, field_col, field_w, cfg.password_len);
        draw_password_field(15, field_col, field_w, 0);
        draw_input_field(17, field_col, field_w, &cfg.hostname, cfg.hostname_len);

        draw_centered(19, "Пользователь получит права администратора sudo (группа wheel).");
        draw_centered(20, "Пароль суперпользователя root совпадает с паролем администратора.");

        if let Some(err) = error_msg {
            draw_info(21, err, FG_RED);
        }

        draw_separator();
        draw_footer(" Enter Следующее поле    Esc Назад ");

        // 1. Имя пользователя
        cfg.username = [0; 64];
        let mut ubuf = [0u8; 64];
        ubuf[..5].copy_from_slice(b"admin");
        cfg.username_len = input_text(11, field_col, &mut ubuf, field_w);
        if cfg.username_len == 0 {
            cfg.username[..5].copy_from_slice(b"admin");
            cfg.username_len = 5;
        } else {
            cfg.username[..cfg.username_len].copy_from_slice(&ubuf[..cfg.username_len]);
        }

        // 2. Пароль
        cfg.password = [0; 128];
        cfg.password_len = input_password(13, field_col, &mut cfg.password, field_w);

        // 3. Повтор пароля
        let mut pass2 = [0u8; 128];
        let pass2_len = input_password(15, field_col, &mut pass2, field_w);

        if cfg.password[..cfg.password_len] != pass2[..pass2_len] {
            error_msg = Some("Пароли не совпадают! Повторите ввод пароля.");
            cfg.password_len = 0;
            continue;
        }

        // 4. Hostname
        cfg.hostname = [0; 64];
        let mut hbuf = [0u8; 64];
        hbuf[..5].copy_from_slice(b"wunix");
        cfg.hostname_len = input_text(17, field_col, &mut hbuf, field_w);
        if cfg.hostname_len == 0 {
            cfg.hostname[..5].copy_from_slice(b"wunix");
            cfg.hostname_len = 5;
        } else {
            cfg.hostname[..cfg.hostname_len].copy_from_slice(&hbuf[..cfg.hostname_len]);
        }

        return Some(cfg);
    }
}
