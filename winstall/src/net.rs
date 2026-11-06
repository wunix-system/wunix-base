// Обнаружение адаптеров, конфигурация сети (DHCP / Static) и ICMP тестирование.

use libwunix::{
    clock_gettime, close, nanosleep, open, pread64, pwrite64, recvfrom, resolve_host_with_server,
    sendto, socket, SockAddrIn, AF_INET, IPPROTO_ICMP, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY,
    SOCK_RAW,
};

pub const MAX_ADAPTERS: usize = 4;

#[derive(Clone, Copy)]
pub struct NetAdapterInfo {
    pub name: [u8; 16],
    pub name_len: usize,
    pub mac: [u8; 18],
    pub mac_len: usize,
    pub is_up: bool,
}

impl Default for NetAdapterInfo {
    fn default() -> Self {
        Self {
            name: [0; 16],
            name_len: 0,
            mac: [0; 18],
            mac_len: 0,
            is_up: true,
        }
    }
}

impl NetAdapterInfo {
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("eth0")
    }

    pub fn mac_str(&self) -> &str {
        core::str::from_utf8(&self.mac[..self.mac_len]).unwrap_or("00:00:00:00:00:00")
    }
}

#[derive(Clone, Copy)]
pub struct NetConfig {
    pub adapter_name: [u8; 16],
    pub adapter_name_len: usize,
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
            adapter_name: [0; 16],
            adapter_name_len: 0,
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
        cfg.set_adapter("eth0");
        cfg.set_ip("10.0.2.15");
        cfg.set_netmask("255.255.255.0");
        cfg.set_gateway("10.0.2.2");
        cfg.set_dns("10.0.2.3");
        cfg
    }
}

impl NetConfig {
    #[allow(dead_code)]
    pub fn adapter_str(&self) -> &str {
        core::str::from_utf8(&self.adapter_name[..self.adapter_name_len]).unwrap_or("eth0")
    }

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

    pub fn set_adapter(&mut self, s: &str) {
        let l = s.len().min(15);
        self.adapter_name[..l].copy_from_slice(&s.as_bytes()[..l]);
        self.adapter_name_len = l;
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

fn read_file_trimmed(path: &[u8], out: &mut [u8]) -> usize {
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

/// Динамическое обнаружение сетевых интерфейсов через /proc/net/dev и /sys/class/net/
pub fn scan_network_adapters(out: &mut [NetAdapterInfo; MAX_ADAPTERS]) -> usize {
    let mut dev_buf = [0u8; 1024];
    let n = read_file_trimmed(b"/proc/net/dev\0", &mut dev_buf);
    let mut count = 0;

    if n > 0 {
        if let Ok(text) = core::str::from_utf8(&dev_buf[..n]) {
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(pos) = trimmed.find(':') {
                    let iface = trimmed[..pos].trim();
                    if iface == "lo" || iface.is_empty() {
                        continue;
                    }
                    if count < MAX_ADAPTERS {
                        let mut info = NetAdapterInfo::default();
                        let l = iface.len().min(15);
                        info.name[..l].copy_from_slice(&iface.as_bytes()[..l]);
                        info.name_len = l;

                        // MAC-адрес
                        let mut pbuf = [0u8; 64];
                        let mut ppos = 0;
                        for &b in b"/sys/class/net/" { pbuf[ppos] = b; ppos += 1; }
                        for &b in &info.name[..info.name_len] { pbuf[ppos] = b; ppos += 1; }
                        for &b in b"/address\0" { pbuf[ppos] = b; ppos += 1; }

                        let mut mac_buf = [0u8; 32];
                        let ml = read_file_trimmed(&pbuf[..ppos], &mut mac_buf);
                        if ml > 0 {
                            let mcopy = ml.min(18);
                            info.mac[..mcopy].copy_from_slice(&mac_buf[..mcopy]);
                            info.mac_len = mcopy;
                        }

                        // Operstate
                        ppos = 0;
                        for &b in b"/sys/class/net/" { pbuf[ppos] = b; ppos += 1; }
                        for &b in &info.name[..info.name_len] { pbuf[ppos] = b; ppos += 1; }
                        for &b in b"/operstate\0" { pbuf[ppos] = b; ppos += 1; }
                        let mut state_buf = [0u8; 16];
                        let sl = read_file_trimmed(&pbuf[..ppos], &mut state_buf);
                        if sl > 0 && &state_buf[..sl] == b"down" {
                            info.is_up = false;
                        }

                        out[count] = info;
                        count += 1;
                    }
                }
            }
        }
    }

    // Запасной опрос: если /proc/net/dev не дал результатов, проверяем eth0 в sysfs
    if count == 0 {
        let mut pbuf = [0u8; 32];
        let ml = read_file_trimmed(b"/sys/class/net/eth0/address\0", &mut pbuf);
        if ml > 0 {
            let mut info = NetAdapterInfo::default();
            info.name[..4].copy_from_slice(b"eth0");
            info.name_len = 4;
            let mcopy = ml.min(18);
            info.mac[..mcopy].copy_from_slice(&pbuf[..mcopy]);
            info.mac_len = mcopy;
            out[0] = info;
            count = 1;
        }
    }

    count
}

/// Считывает реальные активные параметры интерфейса из ядра (/sys/class/net/<iface>/)
pub fn detect_adapter_network(iface: &str) -> NetConfig {
    let mut cfg = NetConfig::default();
    cfg.set_adapter(iface);

    let mut pbuf = [0u8; 64];
    let make_path = |param: &[u8], out: &mut [u8]| -> usize {
        let mut pos = 0;
        for &b in b"/sys/class/net/" { out[pos] = b; pos += 1; }
        for &b in iface.as_bytes() { out[pos] = b; pos += 1; }
        out[pos] = b'/'; pos += 1;
        for &b in param { out[pos] = b; pos += 1; }
        out[pos] = 0; pos += 1;
        pos
    };

    let mut buf = [0u8; 32];

    make_path(b"ipv4_address", &mut pbuf);
    let l = read_file_trimmed(&pbuf, &mut buf);
    if l > 0 && &buf[..l] != b"0.0.0.0" {
        cfg.ip[..l].copy_from_slice(&buf[..l]);
        cfg.ip_len = l;
    }

    make_path(b"ipv4_netmask", &mut pbuf);
    let l = read_file_trimmed(&pbuf, &mut buf);
    if l > 0 && &buf[..l] != b"0.0.0.0" {
        cfg.netmask[..l].copy_from_slice(&buf[..l]);
        cfg.netmask_len = l;
    }

    make_path(b"ipv4_gateway", &mut pbuf);
    let l = read_file_trimmed(&pbuf, &mut buf);
    if l > 0 && &buf[..l] != b"0.0.0.0" {
        cfg.gateway[..l].copy_from_slice(&buf[..l]);
        cfg.gateway_len = l;
    }

    make_path(b"ipv4_dns", &mut pbuf);
    let l = read_file_trimmed(&pbuf, &mut buf);
    if l > 0 && &buf[..l] != b"0.0.0.0" {
        cfg.dns[..l].copy_from_slice(&buf[..l]);
        cfg.dns_len = l;
    }

    cfg
}

/// Сохраняет сетевые параметры в /mnt/etc/
pub fn apply_network_config(cfg: &NetConfig, hostname: &str) {
    let mut conf_buf = [0u8; 512];
    let mut pos = 0;

    let p1 = b"# /etc/network.conf\n[";
    conf_buf[pos..pos + p1.len()].copy_from_slice(p1); pos += p1.len();
    conf_buf[pos..pos + cfg.adapter_name_len].copy_from_slice(&cfg.adapter_name[..cfg.adapter_name_len]); pos += cfg.adapter_name_len;
    let p2 = b"]\nproto=";
    conf_buf[pos..pos + p2.len()].copy_from_slice(p2); pos += p2.len();

    let proto = if cfg.is_dhcp { b"dhcp\n" as &[u8] } else { b"static\n" as &[u8] };
    conf_buf[pos..pos + proto.len()].copy_from_slice(proto); pos += proto.len();

    let p3 = b"ip=";
    conf_buf[pos..pos + p3.len()].copy_from_slice(p3); pos += p3.len();
    conf_buf[pos..pos + cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]); pos += cfg.ip_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p4 = b"netmask=";
    conf_buf[pos..pos + p4.len()].copy_from_slice(p4); pos += p4.len();
    conf_buf[pos..pos + cfg.netmask_len].copy_from_slice(&cfg.netmask[..cfg.netmask_len]); pos += cfg.netmask_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p5 = b"gateway=";
    conf_buf[pos..pos + p5.len()].copy_from_slice(p5); pos += p5.len();
    conf_buf[pos..pos + cfg.gateway_len].copy_from_slice(&cfg.gateway[..cfg.gateway_len]); pos += cfg.gateway_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let p6 = b"dns=";
    conf_buf[pos..pos + p6.len()].copy_from_slice(p6); pos += p6.len();
    conf_buf[pos..pos + cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]); pos += cfg.dns_len;
    conf_buf[pos] = b'\n'; pos += 1;

    let fd = open(b"/mnt/etc/network.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = pwrite64(fd as u64, &conf_buf[..pos], 0);
        close(fd as u64);
    }

    // /mnt/etc/resolv.conf
    let mut res_buf = [0u8; 256];
    let mut rpos = 0;
    if cfg.dns_len > 0 {
        let rp = b"nameserver ";
        res_buf[rpos..rpos + rp.len()].copy_from_slice(rp); rpos += rp.len();
        res_buf[rpos..rpos + cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]); rpos += cfg.dns_len;
        res_buf[rpos] = b'\n'; rpos += 1;
    }
    let fallback_dns = b"nameserver 10.0.2.3\nnameserver 1.1.1.1\nnameserver 8.8.8.8\n";
    res_buf[rpos..rpos + fallback_dns.len()].copy_from_slice(fallback_dns);
    rpos += fallback_dns.len();

    let fd_res = open(b"/mnt/etc/resolv.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd_res >= 0 {
        let _ = pwrite64(fd_res as u64, &res_buf[..rpos], 0);
        close(fd_res as u64);
    }

    // /mnt/etc/hosts
    let mut hosts_buf = [0u8; 256];
    let mut hpos = 0;
    let hp1 = b"127.0.0.1  localhost\n10.0.2.2   host.qemu host gateway\n::1        localhost\n";
    hosts_buf[hpos..hpos + hp1.len()].copy_from_slice(hp1); hpos += hp1.len();
    if cfg.ip_len > 0 {
        hosts_buf[hpos..hpos + cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]); hpos += cfg.ip_len;
    } else {
        let def_ip = b"127.0.1.1";
        hosts_buf[hpos..hpos + def_ip.len()].copy_from_slice(def_ip); hpos += def_ip.len();
    }
    hosts_buf[hpos] = b' '; hpos += 1;
    hosts_buf[hpos] = b' '; hpos += 1;
    let hname = hostname.as_bytes();
    let hlen = hname.len().min(64);
    hosts_buf[hpos..hpos + hlen].copy_from_slice(&hname[..hlen]); hpos += hlen;
    hosts_buf[hpos] = b'\n'; hpos += 1;

    let fd_h = open(b"/mnt/etc/hosts\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd_h >= 0 {
        let _ = pwrite64(fd_h as u64, &hosts_buf[..hpos], 0);
        close(fd_h as u64);
    }
}

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

/// Отправляет один ICMP-пакет на указанный IPv4 и ожидает ответ
pub fn ping_one_packet(fd: i32, target_ip: [u8; 4], seq: u16) -> Option<u64> {
    let dest = SockAddrIn {
        sin_family: AF_INET,
        sin_port: 0,
        sin_addr: u32::from_be_bytes(target_ip),
        sin_zero: [0; 8],
    };

    let ident: u16 = 0xAA55;
    let mut pkt = [0u8; 64];
    pkt[0] = 8; // Echo Request
    pkt[1] = 0;
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
        return None;
    }

    let mut rx_buf = [0u8; 128];
    for _ in 0..30 {
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

        if icmp_type == 0 && (r_id == ident || r_id == 0) && (r_seq == seq || r_seq == 0) {
            let rtt = (now_ms().saturating_sub(t_start)).max(1);
            return Some(rtt);
        }
        sleep_ms(10);
    }

    None
}

/// Результат проверки сайта (2 пакета)
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct HostPingResult {
    pub host: &'static str,
    pub ip: [u8; 4],
    pub dns_resolved: bool,
    pub rtt1: Option<u64>,
    pub rtt2: Option<u64>,
}

/// Проверка соединения: отправляет 4 пакета — 2 на ya.ru и 2 на google.com
pub fn run_ping_four_packets<F>(
    cfg: &NetConfig,
    mut on_step: F,
) -> (HostPingResult, HostPingResult)
where
    F: FnMut(&str, usize, usize, Option<u64>),
{
    let dns_ip = parse_ipv4(cfg.dns_str()).unwrap_or([10, 0, 2, 3]);

    let mut res_ya = HostPingResult {
        host: "ya.ru",
        ip: [0, 0, 0, 0],
        dns_resolved: false,
        rtt1: None,
        rtt2: None,
    };

    let mut res_goog = HostPingResult {
        host: "google.com",
        ip: [0, 0, 0, 0],
        dns_resolved: false,
        rtt1: None,
        rtt2: None,
    };

    let fd = socket(AF_INET, SOCK_RAW as u32, IPPROTO_ICMP);

    // 1. Резолв ya.ru
    if let Ok(ip) = resolve_host_with_server("ya.ru", dns_ip) {
        res_ya.ip = ip;
        res_ya.dns_resolved = true;
    } else {
        // Запасной IP для ya.ru (Yandex DNS / fallback)
        res_ya.ip = [77, 88, 55, 242];
    }

    // 2. Резолв google.com
    if let Ok(ip) = resolve_host_with_server("google.com", dns_ip) {
        res_goog.ip = ip;
        res_goog.dns_resolved = true;
    } else {
        // Запасной IP для google.com (Google DNS)
        res_goog.ip = [8, 8, 8, 8];
    }

    if fd >= 0 {
        // Пакет 1 na ya.ru
        let r1 = ping_one_packet(fd as i32, res_ya.ip, 1);
        res_ya.rtt1 = r1;
        on_step("ya.ru", 1, 4, r1);
        sleep_ms(30);

        // Пакет 2 na ya.ru
        let r2 = ping_one_packet(fd as i32, res_ya.ip, 2);
        res_ya.rtt2 = r2;
        on_step("ya.ru", 2, 4, r2);
        sleep_ms(30);

        // Пакет 3 na google.com
        let r3 = ping_one_packet(fd as i32, res_goog.ip, 3);
        res_goog.rtt1 = r3;
        on_step("google.com", 3, 4, r3);
        sleep_ms(30);

        // Пакет 4 na google.com
        let r4 = ping_one_packet(fd as i32, res_goog.ip, 4);
        res_goog.rtt2 = r4;
        on_step("google.com", 4, 4, r4);

        close(fd as u64);
    } else {
        on_step("ya.ru", 1, 4, None);
        on_step("ya.ru", 2, 4, None);
        on_step("google.com", 3, 4, None);
        on_step("google.com", 4, 4, None);
    }

    (res_ya, res_goog)
}
