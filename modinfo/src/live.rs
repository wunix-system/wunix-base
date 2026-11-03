use libwunix::{close, open, read};

pub struct LiveModuleStatus<'a> {
    pub is_live: bool,
    pub size: u64,
    pub instances: u64,
    pub deps: &'a str,
}

pub fn parse_u64(s: &str) -> u64 {
    let mut val = 0u64;
    for b in s.bytes() {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u64;
        } else {
            break;
        }
    }
    val
}

pub fn query_live_module<'a>(drv_name: &str, buf: &'a mut [u8]) -> LiveModuleStatus<'a> {
    let mut mod_len = 0usize;
    let fd = open(b"/proc/modules\0", 0, 0);
    if fd >= 0 {
        let n = read(fd as u64, buf);
        if n > 0 {
            mod_len = n as usize;
        }
        close(fd as u64);
    }

    let mod_text = core::str::from_utf8(&buf[..mod_len]).unwrap_or("");
    for line in mod_text.lines() {
        let mut parts = line.split_whitespace();
        if let Some(name) = parts.next() {
            if name == drv_name {
                let size = parts.next().map(parse_u64).unwrap_or(0);
                let inst = parts.next().map(parse_u64).unwrap_or(0);
                let deps = parts.next().unwrap_or("-");
                return LiveModuleStatus {
                    is_live: true,
                    size,
                    instances: inst,
                    deps,
                };
            }
        }
    }

    LiveModuleStatus {
        is_live: false,
        size: 0,
        instances: 0,
        deps: "-",
    }
}
