use libwunix::println;

use crate::iface::NetIface;

pub fn show_addr(brief: bool, interfaces: &[NetIface], count: usize) {
    if brief {
        for iface in &interfaces[..count] {
            println!(
                "{:<16} {:<14} {}/{}",
                iface.name_str(),
                iface.operstate_str(),
                iface.ip_str(),
                iface.prefix_str()
            );
        }
        return;
    }

    for (idx, iface) in interfaces[..count].iter().enumerate() {
        let if_idx = idx + 1;
        let name = iface.name_str();
        let mtu = iface.mtu_str();
        let state = iface.operstate_str();
        let mac = iface.mac_str();
        let ip = iface.ip_str();
        let prefix = iface.prefix_str();
        let bcast = iface.broadcast_str();

        if iface.is_loopback {
            println!(
                "{}: {}: <LOOPBACK,UP,LOWER_UP> mtu {} qdisc noqueue state {} group default",
                if_idx, name, mtu, state
            );
            println!("    link/loopback {} brd 00:00:00:00:00:00", mac);
            if !ip.is_empty() {
                println!(
                    "    inet {}/{} scope host lo\n       valid_lft forever preferred_lft forever",
                    ip, prefix
                );
            }
        } else {
            println!(
                "{}: {}: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu {} qdisc pfifo_fast state {} group default",
                if_idx, name, mtu, state
            );
            println!("    link/ether {} brd ff:ff:ff:ff:ff:ff", mac);
            if !ip.is_empty() {
                println!(
                    "    inet {}/{} brd {} scope global {}\n       valid_lft forever preferred_lft forever",
                    ip, prefix, bcast, name
                );
            }
        }
    }
}

pub fn show_link(brief: bool, interfaces: &[NetIface], count: usize) {
    if brief {
        for iface in &interfaces[..count] {
            let flags = if iface.is_loopback { "LOOPBACK,UP,LOWER_UP" } else { "BROADCAST,MULTICAST,UP,LOWER_UP" };
            println!(
                "{:<16} {:<14} {} <{}>",
                iface.name_str(),
                iface.operstate_str(),
                iface.mac_str(),
                flags
            );
        }
        return;
    }

    for (idx, iface) in interfaces[..count].iter().enumerate() {
        let if_idx = idx + 1;
        let name = iface.name_str();
        let mtu = iface.mtu_str();
        let state = iface.operstate_str();
        let mac = iface.mac_str();

        if iface.is_loopback {
            println!(
                "{}: {}: <LOOPBACK,UP,LOWER_UP> mtu {} qdisc noqueue state {} mode DEFAULT group default",
                if_idx, name, mtu, state
            );
            println!("    link/loopback {} brd 00:00:00:00:00:00", mac);
        } else {
            println!(
                "{}: {}: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu {} qdisc pfifo_fast state {} mode DEFAULT group default",
                if_idx, name, mtu, state
            );
            println!("    link/ether {} brd ff:ff:ff:ff:ff:ff", mac);
        }
    }
}
