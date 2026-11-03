use crate::util::read_file_str;

#[derive(Clone, Copy, Default)]
pub struct NetIface {
    pub name: [u8; 16],
    pub name_len: usize,
    pub mac: [u8; 18],
    pub mac_len: usize,
    pub ip: [u8; 16],
    pub ip_len: usize,
    pub prefix: [u8; 4],
    pub prefix_len: usize,
    pub broadcast: [u8; 16],
    pub broadcast_len: usize,
    pub mtu: [u8; 8],
    pub mtu_len: usize,
    pub operstate: [u8; 16],
    pub operstate_len: usize,
    pub is_loopback: bool,
}

impl NetIface {
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }

    pub fn mac_str(&self) -> &str {
        core::str::from_utf8(&self.mac[..self.mac_len]).unwrap_or("00:00:00:00:00:00")
    }

    pub fn ip_str(&self) -> &str {
        core::str::from_utf8(&self.ip[..self.ip_len]).unwrap_or("")
    }

    pub fn prefix_str(&self) -> &str {
        core::str::from_utf8(&self.prefix[..self.prefix_len]).unwrap_or("24")
    }

    pub fn broadcast_str(&self) -> &str {
        core::str::from_utf8(&self.broadcast[..self.broadcast_len]).unwrap_or("")
    }

    pub fn mtu_str(&self) -> &str {
        core::str::from_utf8(&self.mtu[..self.mtu_len]).unwrap_or(if self.is_loopback { "65536" } else { "1500" })
    }

    pub fn operstate_str(&self) -> &str {
        core::str::from_utf8(&self.operstate[..self.operstate_len]).unwrap_or(if self.is_loopback { "UNKNOWN" } else { "UP" })
    }
}

pub fn copy_slice_capped(dest: &mut [u8], src: &[u8]) -> usize {
    let copy_len = src.len().min(dest.len());
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    copy_len
}

pub fn build_sysfs_path(prefix: &[u8], iface_name: &[u8], suffix: &[u8], out: &mut [u8; 128]) -> usize {
    let mut len = 0;
    out[len..len + prefix.len()].copy_from_slice(prefix);
    len += prefix.len();
    out[len..len + iface_name.len()].copy_from_slice(iface_name);
    len += iface_name.len();
    out[len..len + suffix.len()].copy_from_slice(suffix);
    len += suffix.len();
    out[len] = 0;
    len
}

/// Динамически опрашивает ядро через /proc/net/dev и /sys/class/net/
/// Собирает реальный список сетевых интерфейсов
pub fn load_network_interfaces(interfaces: &mut [NetIface; 8]) -> usize {
    let mut dev_buf = [0u8; 2048];
    let dev_text = read_file_str(b"/proc/net/dev\0", &mut dev_buf);

    let mut iface_count = 0usize;
    let mut path_buf = [0u8; 128];
    let mut val_buf = [0u8; 64];

    for line in dev_text.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(':') {
            let iface_name = trimmed[..pos].trim();
            if iface_name.is_empty() || iface_count >= interfaces.len() {
                continue;
            }

            let iface = &mut interfaces[iface_count];
            *iface = NetIface::default();

            let name_bytes = iface_name.as_bytes();
            iface.name_len = copy_slice_capped(&mut iface.name, name_bytes);
            iface.is_loopback = iface_name == "lo";

            // 1. MAC адрес
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/address", &mut path_buf);
            let mac_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !mac_str.is_empty() {
                iface.mac_len = copy_slice_capped(&mut iface.mac, mac_str.as_bytes());
            }

            // 2. IPv4 адрес
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/ipv4_address", &mut path_buf);
            let ip_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !ip_str.is_empty() {
                iface.ip_len = copy_slice_capped(&mut iface.ip, ip_str.as_bytes());
            }

            // 3. Префикс CIDR
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/ipv4_prefix", &mut path_buf);
            let prefix_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !prefix_str.is_empty() {
                iface.prefix_len = copy_slice_capped(&mut iface.prefix, prefix_str.as_bytes());
            }

            // 4. Broadcast адрес
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/ipv4_broadcast", &mut path_buf);
            let bcast_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !bcast_str.is_empty() {
                iface.broadcast_len = copy_slice_capped(&mut iface.broadcast, bcast_str.as_bytes());
            }

            // 5. MTU
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/mtu", &mut path_buf);
            let mtu_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !mtu_str.is_empty() {
                iface.mtu_len = copy_slice_capped(&mut iface.mtu, mtu_str.as_bytes());
            }

            // 6. Operstate
            let path_len = build_sysfs_path(b"/sys/class/net/", name_bytes, b"/operstate", &mut path_buf);
            let op_str = read_file_str(&path_buf[..path_len + 1], &mut val_buf).trim();
            if !op_str.is_empty() {
                let mut up_bytes = [0u8; 16];
                let copy_len = op_str.len().min(up_bytes.len());
                for (k, &b) in op_str.as_bytes()[..copy_len].iter().enumerate() {
                    up_bytes[k] = if (b'a'..=b'z').contains(&b) { b - 32 } else { b };
                }
                iface.operstate_len = copy_slice_capped(&mut iface.operstate, &up_bytes[..copy_len]);
            }

            iface_count += 1;
        }
    }

    iface_count
}
