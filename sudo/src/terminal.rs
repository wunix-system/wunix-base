#![cfg_attr(test, allow(dead_code))]

use libwunix::read_password_masked;

/// Считывает пароль пользователя для sudo с отображением звёздочек '*' вместо скрытого ввода.
/// Возвращает None, если пользователь прервал ввод через Ctrl+C.
pub fn read_password_line<'a>(buf: &'a mut [u8]) -> Option<&'a str> {
    read_password_masked(buf)
}
