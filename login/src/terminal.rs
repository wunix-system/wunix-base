#[allow(unused_imports)]
pub use libwunix::terminal::{
    pop_last_char_bytes, read_line as read_username, read_password_masked, LineMode,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_last_char_ascii() {
        let mut buf = [0u8; 16];
        buf[0] = b'a';
        buf[1] = b'b';
        let mut total = 2;

        assert!(pop_last_char_bytes(&buf, &mut total));
        assert_eq!(total, 1);
        assert_eq!(&buf[..total], b"a");

        assert!(pop_last_char_bytes(&buf, &mut total));
        assert_eq!(total, 0);

        assert!(!pop_last_char_bytes(&buf, &mut total));
    }

    #[test]
    fn test_pop_last_char_utf8() {
        let mut buf = [0u8; 16];
        // Буква 'я' в UTF-8: 0xD1, 0x8F
        buf[0] = 0xD1;
        buf[1] = 0x8F;
        let mut total = 2;

        assert!(pop_last_char_bytes(&buf, &mut total));
        assert_eq!(total, 0);
    }
}
