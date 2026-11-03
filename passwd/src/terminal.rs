use libwunix::read_password_masked;

/// Считывает пароль с отображением маскирующего символа '*' вместо скрытого ввода.
pub fn read_password_line(buf: &mut [u8]) -> &str {
    read_password_masked(buf).unwrap_or("")
}
