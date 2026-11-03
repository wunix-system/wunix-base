// Разбор спецификатора владельца и группы: user:group, user, :group

use libwunix::{get_group_by_name, get_passwd_by_name};

fn parse_u32(s: &str) -> Option<u32> {
    let mut val = 0u32;
    if s.is_empty() {
        return None;
    }
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        val = val.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(val)
}

pub fn resolve_user(user_str: &str) -> Option<u32> {
    if let Some(uid) = parse_u32(user_str) {
        return Some(uid);
    }
    if let Ok(Some(entry)) = get_passwd_by_name(user_str) {
        return Some(entry.uid);
    }
    None
}

pub fn resolve_group(group_str: &str) -> Option<u32> {
    if let Some(gid) = parse_u32(group_str) {
        return Some(gid);
    }
    if let Ok(Some(entry)) = get_group_by_name(group_str) {
        return Some(entry.gid);
    }
    None
}

pub fn parse_owner_spec(spec: &str) -> Result<(Option<u32>, Option<u32>), &'static str> {
    if spec.is_empty() {
        return Err("empty specifier");
    }

    if let Some(colon_idx) = spec.find(':') {
        let u_part = &spec[..colon_idx];
        let g_part = &spec[colon_idx + 1..];

        let target_uid = if !u_part.is_empty() {
            match resolve_user(u_part) {
                Some(uid) => Some(uid),
                None => return Err("invalid user"),
            }
        } else {
            None
        };

        let target_gid = if !g_part.is_empty() {
            match resolve_group(g_part) {
                Some(gid) => Some(gid),
                None => return Err("invalid group"),
            }
        } else if let Some(uid) = target_uid {
            // Если указано "user:", устанавливаем основную группу пользователя
            if let Ok(Some(e)) = libwunix::get_passwd_by_uid(uid) {
                Some(e.gid)
            } else {
                None
            }
        } else {
            None
        };

        Ok((target_uid, target_gid))
    } else {
        // Указан только пользователь
        match resolve_user(spec) {
            Some(uid) => Ok((Some(uid), None)),
            None => Err("invalid user"),
        }
    }
}
