// Разбор режима доступа chmod: восьмеричного (755, 4755) или символьного (+x, -w, u+x, a+rx)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSpec<'a> {
    Octal(u32),
    Symbolic(&'a str),
}

pub fn parse_mode_spec(s: &str) -> Option<ModeSpec<'_>> {
    if s.is_empty() {
        return None;
    }

    // Если строка состоит целиком из восьмеричных цифр
    if s.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        let mut val = 0u32;
        for b in s.bytes() {
            val = val.checked_mul(8)?.checked_add((b - b'0') as u32)?;
        }
        return Some(ModeSpec::Octal(val));
    }

    // Символьный режим: [+|-|=] или [ugoa][+|-|=]
    Some(ModeSpec::Symbolic(s))
}

pub fn apply_symbolic_mode(current_mode: u32, spec: &str) -> Option<u32> {
    let bytes = spec.as_bytes();
    let mut i = 0;
    let mut who_mask = 0o777u32; // по умолчанию 'a' (все пользователи)

    if bytes[0] == b'u' || bytes[0] == b'g' || bytes[0] == b'o' || bytes[0] == b'a' {
        match bytes[0] {
            b'u' => who_mask = 0o700,
            b'g' => who_mask = 0o070,
            b'o' => who_mask = 0o007,
            b'a' => who_mask = 0o777,
            _ => {}
        }
        i += 1;
    }

    if i >= bytes.len() {
        return None;
    }

    let op = bytes[i];
    if op != b'+' && op != b'-' && op != b'=' {
        return None;
    }
    i += 1;

    let mut perm_bits = 0u32;
    while i < bytes.len() {
        match bytes[i] {
            b'r' => {
                if who_mask & 0o700 != 0 { perm_bits |= 0o400; }
                if who_mask & 0o070 != 0 { perm_bits |= 0o040; }
                if who_mask & 0o007 != 0 { perm_bits |= 0o004; }
            }
            b'w' => {
                if who_mask & 0o700 != 0 { perm_bits |= 0o200; }
                if who_mask & 0o070 != 0 { perm_bits |= 0o020; }
                if who_mask & 0o007 != 0 { perm_bits |= 0o002; }
            }
            b'x' => {
                if who_mask & 0o700 != 0 { perm_bits |= 0o100; }
                if who_mask & 0o070 != 0 { perm_bits |= 0o010; }
                if who_mask & 0o007 != 0 { perm_bits |= 0o001; }
            }
            b's' => {
                if who_mask & 0o700 != 0 { perm_bits |= 0o4000; }
                if who_mask & 0o070 != 0 { perm_bits |= 0o2000; }
            }
            b't' => {
                perm_bits |= 0o1000;
            }
            _ => return None,
        }
        i += 1;
    }

    let mut result = current_mode;
    match op {
        b'+' => result |= perm_bits,
        b'-' => result &= !perm_bits,
        b'=' => {
            result &= !who_mask;
            result |= perm_bits;
        }
        _ => return None,
    }

    Some(result & 0o7777)
}
