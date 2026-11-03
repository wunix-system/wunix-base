use crate::format::print_id_token;
use libwunix::{
    for_each_group, getegid, geteuid, getgid, getuid, get_group_by_gid, get_passwd_by_uid, write,
    STDOUT_FILENO,
};

pub fn run_id(specified_user: Option<&str>) {
    let (uid, gid, euid, egid, username) = if let Some(u) = specified_user {
        match libwunix::get_passwd_by_name(u) {
            Ok(Some(entry)) => (entry.uid, entry.gid, entry.uid, entry.gid, Some(entry)),
            _ => {
                let err_msg = libwunix::t("id: пользователь '", "id: user '");
                let not_found = libwunix::t("': не найден\n", "': no such user\n");
                libwunix::write(libwunix::STDERR_FILENO, err_msg.as_bytes());
                libwunix::write(libwunix::STDERR_FILENO, u.as_bytes());
                libwunix::write(libwunix::STDERR_FILENO, not_found.as_bytes());
                libwunix::exit(1);
            }
        }
    } else {
        let u = getuid();
        let g = getgid();
        let eu = geteuid();
        let eg = getegid();
        let entry = get_passwd_by_uid(u).ok().flatten();
        (u, g, eu, eg, entry)
    };

    let u_name = username.as_ref().map(|e| e.username()).unwrap_or("");
    print_id_token("uid=", uid, u_name);

    let g_entry = get_group_by_gid(gid).ok().flatten();
    let g_name = g_entry.as_ref().map(|g| g.name()).unwrap_or("");
    print_id_token(" gid=", gid, g_name);

    if euid != uid {
        let eu_entry = get_passwd_by_uid(euid).ok().flatten();
        let eu_name = eu_entry.as_ref().map(|e| e.username()).unwrap_or("");
        print_id_token(" euid=", euid, eu_name);
    }

    if egid != gid {
        let eg_entry = get_group_by_gid(egid).ok().flatten();
        let eg_name = eg_entry.as_ref().map(|g| g.name()).unwrap_or("");
        print_id_token(" egid=", egid, eg_name);
    }

    write(STDOUT_FILENO, b" groups=");

    // Выводим основную группу
    let mut num_buf = [0u8; 16];
    let gid_str = libwunix::fmt::utoa(gid as u64, &mut num_buf);
    write(STDOUT_FILENO, gid_str.as_bytes());
    if !g_name.is_empty() {
        write(STDOUT_FILENO, b"(");
        write(STDOUT_FILENO, g_name.as_bytes());
        write(STDOUT_FILENO, b")");
    }
    let mut first = false;

    // Ищем дополнительные группы
    let _ = for_each_group(|grp| {
        if grp.gid != gid && !u_name.is_empty() && grp.has_member(u_name) {
            if !first {
                write(STDOUT_FILENO, b",");
            }
            let mut buf = [0u8; 16];
            let s = libwunix::fmt::utoa(grp.gid as u64, &mut buf);
            write(STDOUT_FILENO, s.as_bytes());
            write(STDOUT_FILENO, b"(");
            write(STDOUT_FILENO, grp.name().as_bytes());
            write(STDOUT_FILENO, b")");
            first = false;
        }
        true
    });

    write(STDOUT_FILENO, b"\n");
}
