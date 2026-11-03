use libwunix::{write, STDOUT_FILENO};

pub fn print_id_token(prefix: &str, id: u32, name: &str) {
    write(STDOUT_FILENO, prefix.as_bytes());
    let mut num_buf = [0u8; 16];
    let num_str = libwunix::fmt::utoa(id as u64, &mut num_buf);
    write(STDOUT_FILENO, num_str.as_bytes());
    if !name.is_empty() {
        write(STDOUT_FILENO, b"(");
        write(STDOUT_FILENO, name.as_bytes());
        write(STDOUT_FILENO, b")");
    }
}
