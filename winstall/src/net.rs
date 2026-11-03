// Настройка и проверка сетевого подключения для Wunix OS.
// Поддерживает автоматическую конфигурацию (DHCP) и ручную (Static IP).
// Выполняет валидацию и тестирование соединения через ICMP Echo (ping).

use libwunix::{
    clock_gettime, close, nanosleep, open, pread64, recvfrom,
    sendto, socket, SockAddrIn, AF_INET, IPPROTO_ICMP, O_CREAT, O_RDONLY,
    O_TRUNC, O_WRONLY, SOCK_RAW,
};

#[derive(Clone, Copy)]
pub struct NetConfig {
    pub is_dhcp: bool,
    pub ip: [u8; 16],
    pub ip_len: usize,
    pub netmask: [u8; 16],
    pub netmask_len: usize,
    pub gateway: [u8; 16],
    pub gateway_len: usize,
    pub dns: [u8; 16],
    pub dns_len: usize,
}

impl Default for NetConfig {
    fn default() -> Self {
        let mut cfg = Self {
            is_dhcp: true,
            ip: [0; 16],
            ip_len: 0,
            netmask: [0; 16],
            netmask_len: 0,
            gateway: [0; 16],
            gateway_len: 0,
            dns: [0; 16],
            dns_len: 0,
        };
        cfg.set_ip("10.0.2.15");
        cfg.set_netmask("255.255.255.0");
        cfg.set_gateway("10.0.2.2");
        cfg.set_dns("10.0.2.3");
        cfg
    }
}

impl NetConfig {
    pub fn ip_str(&self) -> &str {
        core::str::from_utf8(&self.ip[..self.ip_len]).unwrap_or("10.0.2.15")
    }

    pub fn netmask_str(&self) -> &str {
        core::str::from_utf8(&self.netmask[..self.netmask_len]).unwrap_or("255.255.255.0")
    }

    pub fn gateway_str(&self) -> &str {
        core::str::from_utf8(&self.gateway[..self.gateway_len]).unwrap_or("10.0.2.2")
    }

    pub fn dns_str(&self) -> &str {
        core::str::from_utf8(&self.dns[..self.dns_len]).unwrap_or("10.0.2.3")
    }

    pub fn set_ip(&mut self, s: &str) {
        let l = s.len().min(15);
        self.ip[..l].copy_from_slice(&s.as_bytes()[..l]);
        self.ip_len = l;
    }

    pub fn set_netmask(&mut self, s: &str) {
        let l = s.len().min(15);
        self.netmask[..l].copy_from_slice(&s.as_bytes()[..l]);
        self.netmask_len = l;
    }

    pub fn set_gateway(&mut self, s: &str) {
        let l = s.len().min(15);
        self.gateway[..l].copy_from_slice(&s.as_bytes()[..l]);
        self.gateway_len = l;
    }

    pub fn set_dns(&mut self, s: &str) {
        let l = s.len().min(15);
        self.dns[..l].copy_from_slice(&s.as_bytes()[..l]);
        self.dns_len = l;
    }
}

fn read_sysfs_trimmed(path: &[u8], out: &mut [u8]) -> usize {
    let fd = open(path, O_RDONLY as u64, 0);
    if fd < 0 { return 0; }
    let n = pread64(fd as u64, out, 0);
    close(fd as u64);
    if n <= 0 { return 0; }
    let mut len = n as usize;
    while len > 0 && (out[len - 1] == b'\n' || out[len - 1] == b'\r' || out[len - 1] == b' ') {
        len -= 1;
    }
    len
}

/// Считывает текущие активные настройки сети ядра из /sys/class/net/eth0/
pub fn detect_current_network() -> NetConfig {
    let mut cfg = NetConfig::default();

    let mut buf = [0u8; 32];
    let l = read_sysfs_trimmed(b"/sys/class/net/eth0/ipv4_address\0", &mut buf);
    if l > 0 {
        cfg.ip[..l].copy_from_slice(&buf[..l]);
        cfg.ip_len = l;
    }

    let l = read_sysfs_trimmed(b"/sys/class/net/eth0/ipv4_netmask\0", &mut buf);
    if l > 0 {
        cfg.netmask[..l].copy_from_slice(&buf[..l]);
        cfg.netmask_len = l;
    }

    let l = read_sysfs_trimmed(b"/sys/class/net/eth0/ipv4_gateway\0", &mut buf);
    if l > 0 {
        cfg.gateway[..l].copy_from_slice(&buf[..l]);
        cfg.gateway_len = l;
    }

    let l = read_sysfs_trimmed(b"/sys/class/net/eth0/ipv4_dns\0", &mut buf);
    if l > 0 {
        cfg.dns[..l].copy_from_slice(&buf[..l]);
        cfg.dns_len = l;
    }

    cfg
}

/// Сохраняет сетевые настройки на целевую систему (/mnt/etc/network.conf, /mnt/etc/resolv.conf)
/// и обновляет локальный /etc/resolv.conf в текущей Live-системе.
pub fn apply_network_config(cfg: &NetConfig, hostname: &str) {
    // 1. /mnt/etc/network.conf
    let mut conf_buf = [0u8; 512];
    let mut pos = 0;

    let p1 = "# /etc/network.conf — Конфигурация схемы сети Wunix\n[eth0]\nproto=".as_bytes();
    conf_buf[pos..pos + p1.len()].copy_from_slice(p1);
    pos += p1.len();

    let proto = if cfg.is_dhcp { b"dhcp\n" as &[u8] } else { b"static\n" as &[u8] };
    conf_buf[pos..pos + proto.len()].copy_from_slice(proto);
    pos += proto.len();

    let p2 = b"ip=";
    conf_buf[pos..pos + p2.len()].copy_from_slice(p2); pos += p2.len();
    conf_buf[pos..pos + cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]); pos += cfg.ip_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p3 = b"netmask=";
    conf_buf[pos..pos + p3.len()].copy_from_slice(p3); pos += p3.len();
    conf_buf[pos..pos + cfg.netmask_len].copy_from_slice(&cfg.netmask[..cfg.netmask_len]); pos += cfg.netmask_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p4 = b"gateway=";
    conf_buf[pos..pos + p4.len()].copy_from_slice(p4); pos += p4.len();
    conf_buf[pos..pos + cfg.gateway_len].copy_from_slice(&cfg.gateway[..cfg.gateway_len]); pos += cfg.gateway_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p5 = b"dns=";
    conf_buf[pos..pos + p5.len()].copy_from_slice(p5); pos += p5.len();
    conf_buf[pos..pos + cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]); pos += cfg.dns_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let fd = open(b"/mnt/etc/network.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = libwunix::pwrite64(fd as u64, &conf_buf[..pos], 0);
        close(fd as u64);
    }

    // 2. /mnt/etc/resolv.conf
    let mut resolv_buf = [0u8; 256];
    let mut rpos = 0;
    let r1 = "# /etc/resolv.conf — DNS служба Wunix\nnameserver ".as_bytes();
    resolv_buf[rpos..rpos + r1.len()].copy_from_slice(r1); rpos += r1.len();
    resolv_buf[rpos..rpos + cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]); rpos += cfg.dns_len;
    resolv_buf[rpos] = b'\n'; rpos += 1;

    let fd = open(b"/mnt/etc/resolv.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = libwunix::pwrite64(fd as u64, &resolv_buf[..rpos], 0);
        close(fd as u64);
    }

    // Обновляем также текущий /etc/resolv.conf в Live сессии
    let fd = open(b"/etc/resolv.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = libwunix::pwrite64(fd as u64, &resolv_buf[..rpos], 0);
        close(fd as u64);
    }

    // 3. /mnt/etc/hosts
    let mut hosts_buf = [0u8; 256];
    let mut hpos = 0;
    let h1 = b"127.0.0.1   localhost\n::1         localhost\n";
    hosts_buf[hpos..hpos + h1.len()].copy_from_slice(h1); hpos += h1.len();
    hosts_buf[hpos..hpos + cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]); hpos += cfg.ip_len;
    hosts_buf[hpos] = b' '; hpos += 1;
    hosts_buf[hpos] = b' '; hpos += 1;
    let hname = hostname.as_bytes();
    let hlen = hname.len().min(64);
    hosts_buf[hpos..hpos + hlen].copy_from_slice(&hname[..hlen]); hpos += hlen;
    hosts_buf[hpos] = b'\n'; hpos += 1;

    let fd = open(b"/mnt/etc/hosts\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = libwunix::pwrite64(fd as u64, &hosts_buf[..hpos], 0);
        close(fd as u64);
    }
}

/// Разбирает IPv4 строку вида "192.168.1.1" в 4 байта.
pub fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut idx = 0;
    let mut cur = 0u16;
    let mut has_digit = false;

    for &b in s.as_bytes() {
        if b == b'.' {
            if !has_digit || idx >= 3 || cur > 255 { return None; }
            octets[idx] = cur as u8;
            idx += 1;
            cur = 0;
            has_digit = false;
        } else if b >= b'0' && b <= b'9' {
            cur = cur * 10 + (b - b'0') as u16;
            if cur > 255 { return None; }
            has_digit = true;
        } else {
            return None;
        }
    }
    if !has_digit || idx != 3 || cur > 255 { return None; }
    octets[3] = cur as u8;
    Some(octets)
}

fn icmp_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += ((data[i] as u32) << 8) | (data[i + 1] as u32);
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

#[repr(C)]
#[derive(Default)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

fn now_ms() -> u64 {
    let mut ts = Timespec::default();
    let _ = clock_gettime(1 /* CLOCK_MONOTONIC */, &mut ts as *mut Timespec as *mut u8);
    (ts.tv_sec as u64) * 1000 + (ts.tv_nsec as u64) / 1_000_000
}

fn sleep_ms(ms: u64) {
    let req = [(ms / 1000) as u64, ((ms % 1000) * 1_000_000) as u64];
    let _ = nanosleep(req.as_ptr() as *const u8, core::ptr::null_mut());
}

/// Выполняет быструю серию ICMP Echo запросов до указанного IP-адреса.
/// Поддерживает обработку как чистого ICMP-пакета (начиная с байта 0),
/// так и пакета с 20-байтным IPv4-заголовком.
pub fn ping_target_probes<F>(target_ip: [u8; 4], probes: usize, mut on_probe: F) -> usize
where
    F: FnMut(usize, Option<u64>),
{
    let fd = socket(AF_INET, SOCK_RAW as u32, IPPROTO_ICMP);
    if fd < 0 {
        for s in 1..=probes {
            on_probe(s, None);
        }
        return 0;
    }

    let dest = SockAddrIn {
        sin_family: AF_INET,
        sin_port: 0,
        sin_addr: u32::from_be_bytes(target_ip),
        sin_zero: [0; 8],
    };

    let ident: u16 = 0xAA55;
    let mut successful = 0;

    for seq in 1..=probes {
        let mut pkt = [0u8; 64];
        pkt[0] = 8; // ICMP Type 8: Echo Request
        pkt[1] = 0; // Code 0
        pkt[4] = (ident >> 8) as u8;
        pkt[5] = (ident & 0xFF) as u8;
        pkt[6] = ((seq >> 8) & 0xFF) as u8;
        pkt[7] = (seq & 0xFF) as u8;
        for i in 8..64 {
            pkt[i] = b'a' + (((i - 8) % 23) as u8);
        }

        let cs = icmp_checksum(&pkt);
        pkt[2] = (cs >> 8) as u8;
        pkt[3] = (cs & 0xFF) as u8;

        let t_start = now_ms();
        let sent = sendto(fd as u64, &pkt, 0, Some(&dest));
        if sent < 0 {
            on_probe(seq, None);
            continue;
        }

        let mut rx_buf = [0u8; 128];
        let mut got_reply = false;
        let mut rtt = 0u64;

        // Ожидаем ответ короткими интервалами по 5 мс (для локального шлюза ответ приходит за 1 мс)
        for _ in 0..40 {
            let n = recvfrom(fd as u64, &mut rx_buf, 0, None);
            let (icmp_type, r_id, r_seq) = if n >= 28 && rx_buf[0] == 0x45 {
                (
                    rx_buf[20],
                    u16::from_be_bytes([rx_buf[24], rx_buf[25]]),
                    u16::from_be_bytes([rx_buf[26], rx_buf[27]]),
                )
            } else if n >= 8 {
                (
                    rx_buf[0],
                    u16::from_be_bytes([rx_buf[4], rx_buf[5]]),
                    u16::from_be_bytes([rx_buf[6], rx_buf[7]]),
                )
            } else {
                (255, 0, 0)
            };

            if icmp_type == 0
                && (r_id == ident || r_id == 0)
                && (r_seq as usize == seq || r_seq == 0)
            {
                rtt = (now_ms().saturating_sub(t_start)).max(1);
                got_reply = true;
                break;
            }
            sleep_ms(5);
        }

        if got_reply {
            successful += 1;
            on_probe(seq, Some(rtt));
        } else {
            on_probe(seq, None);
        }

        if seq < probes {
            sleep_ms(40);
        }
    }

    close(fd as u64);
    successful
}


