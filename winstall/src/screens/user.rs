// Создание первого пользователя-администратора (sudo / wheel) и задание имени хоста.

use crate::tui::{
    center_col, clear_screen, draw_centered, draw_footer, draw_header, draw_info,
    draw_input_field, draw_label, draw_logo, draw_password_field, draw_separator,
    hide_cursor, input_password_nav, input_text_nav, InputExit, FG_RED, LOGO_ROW,
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
    let mut cur_field: usize = 0;

    let mut ubuf = [0u8; 64];
    let mut pass1 = [0u8; 128];
    let mut pass1_len = 0;
    let mut pass2 = [0u8; 128];
    let mut pass2_len = 0;
    let mut hbuf = [0u8; 64];

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

        let u_len = ubuf.iter().position(|&b| b == 0).unwrap_or(ubuf.len());
        let h_len = hbuf.iter().position(|&b| b == 0).unwrap_or(hbuf.len());

        draw_input_field(11, field_col, field_w, &ubuf, u_len);
        draw_password_field(13, field_col, field_w, pass1_len);
        draw_password_field(15, field_col, field_w, pass2_len);
        draw_input_field(17, field_col, field_w, &hbuf, h_len);

        draw_centered(19, "Пользователь получит права администратора sudo (группа wheel).");
        draw_centered(20, "Пароль суперпользователя root совпадает с паролем администратора.");

        if let Some(err) = error_msg {
            draw_info(21, err, FG_RED);
        }

        draw_separator();
        draw_footer(" Enter Подтвердить    Tab/↑↓ Переключение полей    Esc Назад ");

        match cur_field {
            0 => {
                let (len, exit) = input_text_nav(11, field_col, &mut ubuf, field_w);
                cfg.username_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 1,
                    InputExit::Prev => cur_field = 3,
                    InputExit::Cancel => return None,
                }
            }
            1 => {
                let (len, exit) = input_password_nav(13, field_col, &mut pass1, field_w);
                pass1_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 2,
                    InputExit::Prev => cur_field = 0,
                    InputExit::Cancel => return None,
                }
            }
            2 => {
                let (len, exit) = input_password_nav(15, field_col, &mut pass2, field_w);
                pass2_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 3,
                    InputExit::Prev => cur_field = 1,
                    InputExit::Cancel => return None,
                }
            }
            3 => {
                let (len, exit) = input_text_nav(17, field_col, &mut hbuf, field_w);
                cfg.hostname_len = len;
                match exit {
                    InputExit::Submit => {
                        if pass1[..pass1_len] != pass2[..pass2_len] {
                            error_msg = Some("Пароли не совпадают! Повторите ввод пароля.");
                            pass1 = [0; 128];
                            pass1_len = 0;
                            pass2 = [0; 128];
                            pass2_len = 0;
                            cur_field = 1;
                            continue;
                        }
                        if cfg.username_len == 0 {
                            cfg.username[..5].copy_from_slice(b"admin");
                            cfg.username_len = 5;
                        } else {
                            cfg.username[..cfg.username_len].copy_from_slice(&ubuf[..cfg.username_len]);
                        }
                        cfg.password[..pass1_len].copy_from_slice(&pass1[..pass1_len]);
                        cfg.password_len = pass1_len;

                        if cfg.hostname_len == 0 {
                            cfg.hostname[..5].copy_from_slice(b"wunix");
                            cfg.hostname_len = 5;
                        } else {
                            cfg.hostname[..cfg.hostname_len].copy_from_slice(&hbuf[..cfg.hostname_len]);
                        }
                        return Some(cfg);
                    }
                    InputExit::Next => cur_field = 0,
                    InputExit::Prev => cur_field = 2,
                    InputExit::Cancel => return None,
                }
            }
            _ => cur_field = 0,
        }
    }
}
