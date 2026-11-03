use libwunix::println;

use crate::iface::NetIface;
use crate::util::{parse_hex_ipv4_le, read_file_str};

pub fn show_route(interfaces: &[NetIface], if_count: usize) {
    let mut route_buf = [0u8; 2048];
    let route_text = read_file_str(b"/proc/net/route\0", &mut route_buf);

    for (line_idx, line) in route_text.lines().enumerate() {
        if line_idx == 0 || line.trim().is_empty() {
            continue; // Пропускаем заголовок таблицы
        }

        let mut fields = line.split_whitespace();
        let iface_field = match fields.next() {
            Some(f) => f,
            None => continue,
        };
        let dest_hex = match fields.next() {
            Some(d) => d,
            None => continue,
        };
        let gw_hex = match fields.next() {
            Some(g) => g,
            None => continue,
        };
        let _flags = fields.next();
        let _refcnt = fields.next();
        let _use = fields.next();
        let _metric = fields.next();
        let mask_hex = match fields.next() {
            Some(m) => m,
            None => continue,
        };

        let dest_ip = parse_hex_ipv4_le(dest_hex).unwrap_or([0, 0, 0, 0]);
        let gw_ip = parse_hex_ipv4_le(gw_hex).unwrap_or([0, 0, 0, 0]);
        let mask = parse_hex_ipv4_le(mask_hex).unwrap_or([0, 0, 0, 0]);

        if dest_ip == [0, 0, 0, 0] {
            // Маршрут по умолчанию (Default gateway)
            println!(
                "default via {}.{}.{}.{} dev {} proto static",
                gw_ip[0], gw_ip[1], gw_ip[2], gw_ip[3], iface_field
            );
        } else {
            // Маршрут локальной подсети
            let prefix: u32 = mask.iter().map(|&b| b.count_ones()).sum();

            // Ищем исходный IP адрес для этого интерфейса
            let mut src_ip = "0.0.0.0";
            for iface in &interfaces[..if_count] {
                if iface.name_str() == iface_field && !iface.ip_str().is_empty() {
                    src_ip = iface.ip_str();
                    break;
                }
            }

            println!(
                "{}.{}.{}.{}/{} dev {} proto kernel scope link src {}",
                dest_ip[0], dest_ip[1], dest_ip[2], dest_ip[3], prefix, iface_field, src_ip
            );
        }
    }
}
