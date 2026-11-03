use crate::terminal::read_password_masked;
use libwunix::{
    get_passwd_by_name, get_shadow_by_name, t, verify_password, write, PasswdEntry,
    STDOUT_FILENO,
};

#[derive(Debug, PartialEq, Eq)]
pub enum AuthResult {
    Success(PasswdEntry),
    Failure,
    Cancelled,
}

/// Выполняет аутентификацию пользователя в системе.
///
/// Архитектурные требования безопасности:
/// 1. Если пароль в /etc/shadow пуст (как у root по умолчанию в Wunix), вход происходит
///    сразу без блокировки запросом пароля.
/// 2. Если пользователь не существует, мы все равно запрашиваем пароль и маскируем его '*',
///    чтобы потенциальный злоумышленник не мог составить словарь валидных логинов по
///    наличию или отсутствию шага ввода пароля.
pub fn authenticate_user(username: &str) -> AuthResult {
    let user_entry = match get_passwd_by_name(username) {
        Ok(Some(entry)) => Some(entry),
        _ => None,
    };

    let shadow_entry = get_shadow_by_name(username).ok().flatten();

    // Быстрый вход для учетных записей без установленного пароля
    if let (Some(user), Some(shadow)) = (&user_entry, &shadow_entry) {
        if shadow.hash().is_empty() {
            return AuthResult::Success(*user);
        }
    }

    // Запрос пароля с маскировкой ввода звездочками '*'
    let prompt_pass = t("Пароль: ", "Password: ");
    write(STDOUT_FILENO, prompt_pass.as_bytes());

    let mut password_buf = [0u8; 128];
    let raw_pwd = match read_password_masked(&mut password_buf) {
        Some(p) => p.trim(),
        None => return AuthResult::Cancelled,
    };

    let user = match user_entry {
        Some(u) => u,
        None => return AuthResult::Failure,
    };

    match verify_password(username, raw_pwd) {
        Ok(_) => AuthResult::Success(user),
        Err(_) => AuthResult::Failure,
    }
}
