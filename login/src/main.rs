#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code, unused))]

mod auth;
mod session;
mod terminal;

use auth::{authenticate_user, AuthResult};
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};
use session::spawn_user_session;
use terminal::read_username;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct LoginConfig<'a> {
    pub force_user: Option<&'a str>,
    pub initial_user: Option<&'a str>,
    pub preserve_env: bool,
}

/// Чистый парсер аргументов командной строки утилиты login.
pub fn parse_args<'a>(args: &'a [&'a str]) -> Result<LoginConfig<'a>, bool> {
    let mut config = LoginConfig::default();
    let mut i = 1;
    while i < args.len() {
        let arg = args[i];
        if arg == "-h" || arg == "--help" {
            return Err(true);
        } else if arg == "-p" {
            config.preserve_env = true;
        } else if arg == "-f" {
            i += 1;
            if i < args.len() {
                config.force_user = Some(args[i]);
            }
        } else if !arg.starts_with('-') && config.initial_user.is_none() {
            config.initial_user = Some(arg);
        }
        i += 1;
    }
    Ok(config)
}

fn print_help() {
    let help_msg = t(
        "Использование: login [-p] [-f пользователь] [пользователь]\n\
         Начинает новую сессию пользователя в системе Wunix.\n\n\
         Опции:\n\
           -f <пользователь>  принудительный вход без запроса пароля (только root)\n\
           -p                 сохранить переменные окружения\n\
           -h, --help         показать эту справку и выйти\n",
        "Usage: login [-p] [-f user] [user]\n\
         Begin a new user session on Wunix.\n\n\
         Options:\n\
           -f <user>          force login without password (root only)\n\
           -p                 preserve environment\n\
           -h, --help         display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

fn get_arg_str<'a>(argv: *const *const u8, idx: isize) -> Option<&'a str> {
    if argv.is_null() {
        return None;
    }
    let p = unsafe { *argv.offset(idx) };
    if p.is_null() {
        return None;
    }
    let len = str_utils::strlen(p);
    let slice = unsafe { core::slice::from_raw_parts(p, len) };
    core::str::from_utf8(slice).ok()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut arg_ptrs = [""; 16];
    let count = (argc as usize).min(16);
    let mut collected = 0;
    for idx in 0..count {
        if let Some(s) = get_arg_str(argv, idx as isize) {
            arg_ptrs[collected] = s;
            collected += 1;
        }
    }

    let config = match parse_args(&arg_ptrs[..collected]) {
        Ok(cfg) => cfg,
        Err(_) => {
            print_help();
            exit(0);
        }
    };

    run_login(config);
}

fn run_login(config: LoginConfig) -> ! {
    let greeting = t(
        "\n\x1b[1;36mДобро пожаловать в Wunix OS (x86_64)!\x1b[0m\n\n",
        "\n\x1b[1;36mWelcome to Wunix OS (x86_64)!\x1b[0m\n\n",
    );
    write(STDOUT_FILENO, greeting.as_bytes());

    // Принудительный вход через -f (только для root)
    if let Some(forced) = config.force_user {
        if libwunix::getuid() == 0 {
            if let Ok(Some(user_entry)) = libwunix::get_passwd_by_name(forced) {
                spawn_user_session(&user_entry, config.preserve_env);
            }
        }
    }

    // Первичная попытка входа, если имя передано позиционным аргументом
    if let Some(user) = config.initial_user {
        match authenticate_user(user) {
            AuthResult::Success(user_entry) => {
                spawn_user_session(&user_entry, config.preserve_env);
            }
            AuthResult::Failure => {
                handle_auth_failure();
            }
            AuthResult::Cancelled => {}
        }
    }

    // Интерактивный цикл аутентификации терминала
    let mut failures = 0;
    loop {
        let prompt_login = t("wunix login: ", "wunix login: ");
        write(STDOUT_FILENO, prompt_login.as_bytes());

        let mut username_buf = [0u8; 64];
        let username = match read_username(&mut username_buf) {
            Some(u) => u.trim(),
            None => {
                write(STDOUT_FILENO, b"\n");
                continue;
            }
        };

        if username.is_empty() {
            continue;
        }

        match authenticate_user(username) {
            AuthResult::Success(user_entry) => {
                spawn_user_session(&user_entry, config.preserve_env);
            }
            AuthResult::Failure => {
                failures += 1;
                handle_auth_failure();
                if failures >= 3 {
                    let max_msg = t(
                        "login: превышено максимальное количество попыток входа.\n",
                        "login: maximum number of login attempts exceeded.\n",
                    );
                    write(STDERR_FILENO, max_msg.as_bytes());
                    exit(1);
                }
            }
            AuthResult::Cancelled => {
                continue;
            }
        }
    }
}

fn handle_auth_failure() {
    let fail = t("Сбой входа в систему\n\n", "Login incorrect\n\n");
    write(STDERR_FILENO, fail.as_bytes());

    let req = [1u64, 0u64];
    libwunix::nanosleep(req.as_ptr() as *const u8, core::ptr::null_mut());
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in login\n");
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_defaults() {
        let args = ["login"];
        let cfg = parse_args(&args).unwrap();
        assert_eq!(cfg.force_user, None);
        assert_eq!(cfg.initial_user, None);
        assert!(!cfg.preserve_env);
    }

    #[test]
    fn test_parse_args_help() {
        let args = ["login", "--help"];
        assert_eq!(parse_args(&args), Err(true));

        let args_short = ["login", "-h"];
        assert_eq!(parse_args(&args_short), Err(true));
    }

    #[test]
    fn test_parse_args_user_and_flags() {
        let args = ["login", "-p", "alice"];
        let cfg = parse_args(&args).unwrap();
        assert_eq!(cfg.initial_user, Some("alice"));
        assert!(cfg.preserve_env);

        let args_force = ["login", "-f", "bob"];
        let cfg_force = parse_args(&args_force).unwrap();
        assert_eq!(cfg_force.force_user, Some("bob"));
    }
}
