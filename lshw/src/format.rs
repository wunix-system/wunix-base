use libwunix::{print, println, t};
use crate::hw::HwInventory;

pub fn is_match(filter: Option<&[u8]>, class_en: &[u8], class_ru: &[u8]) -> bool {
    if let Some(target) = filter {
        if class_en == target || class_ru == target {
            return true;
        }
        if (target == b"cpu" || target == b"processor") && class_en == b"processor" {
            return true;
        }
        if target == b"storage" && class_en == b"disk" {
            return true;
        }
        return false;
    }
    true
}

pub fn render_json(inv: &HwInventory, filter: Option<&[u8]>) {
    println!("[");
    let mut first = true;
    let mut print_json_item = |id: &str, class: &str, dev: &str, desc: &str| {
        if !first {
            println!(",");
        }
        first = false;
        print!(
            "  {{\n    \"id\": \"{}\",\n    \"class\": \"{}\",\n    \"claimed\": true,\n    \"description\": \"{}\"",
            id, class, desc
        );
        if !dev.is_empty() {
            print!(",\n    \"logicalname\": \"{}\"", dev);
        }
        print!("\n  }}");
    };

    if is_match(filter, b"system", b"\xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xb0") {
        print_json_item("/0", "system", "", "Wunix Standard PC (x86_64)");
    }
    if is_match(filter, b"bus", b"\xd1\x88\xd0\xb8\xd0\xbd\xd0\xb0") {
        print_json_item("/0/0", "bus", "", "Motherboard / Intel i440FX");
    }
    if is_match(filter, b"memory", b"\xd0\xbf\xd0\xb0\xd0\xbc\xd1\x8f\xd1\x82\xd1\x8c") {
        print_json_item("/0/1", "memory", "", "System RAM");
    }
    if is_match(filter, b"processor", b"\xd0\xbf\xd1\x80\xd0\xbe\xd1\x86\xd0\xb5\xd1\x81\xd1\x81\xd0\xbe\xd1\x80") {
        print_json_item("/0/2", "processor", "cpu0", inv.cpu_brand);
    }
    if is_match(filter, b"display", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xbf\xd0\xbb\xd0\xb5\xd0\xb9") {
        print_json_item("/0/100/2", "display", "fb0", "Virtio GPU Framebuffer");
    }
    if is_match(filter, b"network", b"\xd1\x81\xd0\xb5\xd1\x82\xd1\x8c") {
        print_json_item("/0/100/3", "network", "eth0", "Virtio Network Adapter");
    }
    if is_match(filter, b"disk", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xba") {
        for i in 0..inv.disk_count {
            let d = &inv.disks[i];
            let d_name = core::str::from_utf8(&d.name[..d.name_len]).unwrap_or("disk");
            print_json_item("/0/100/4", "disk", d_name, "Virtio Block Disk");
        }
    }
    println!("\n]");
}

pub fn render_short(inv: &HwInventory, filter: Option<&[u8]>) {
    println!(
        "{}",
        t(
            "Путь к H/W       Устройство  Класс          Описание",
            "H/W path         Device      Class          Description"
        )
    );
    println!("=========================================================================");

    if is_match(filter, b"system", b"\xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xb0") {
        println!("{:<16} {:<11} {:<14} {}", "/0", "", t("система", "system"), "Wunix Standard PC (x86_64)");
    }
    if is_match(filter, b"bus", b"\xd1\x88\xd0\xb8\xd0\xbd\xd0\xb0") {
        println!("{:<16} {:<11} {:<14} {}", "/0/0", "", t("шина", "bus"), "Motherboard / Intel i440FX");
    }
    if is_match(filter, b"memory", b"\xd0\xbf\xd0\xb0\xd0\xbc\xd1\x8f\xd1\x82\xd1\x8c") {
        let desc_ru = t("МиБ Системная память RAM", "MiB System RAM");
        print!("{:<16} {:<11} {:<14} {}{}", "/0/1", "", t("память", "memory"), inv.total_ram_mb, desc_ru);
        if inv.free_ram_mb > 0 {
            print!(" ({} {})", inv.free_ram_mb, t("МиБ свободно", "MiB free"));
        }
        println!();
    }
    if is_match(filter, b"processor", b"\xd0\xbf\xd1\x80\xd0\xbe\xd1\x86\xd0\xb5\xd1\x81\xd1\x81\xd0\xbe\xd1\x80") {
        println!("{:<16} {:<11} {:<14} {}", "/0/2", "cpu0", t("процессор", "processor"), inv.cpu_brand);
    }
    if is_match(filter, b"bridge", b"\xd0\xbc\xd0\xbe\xd1\x81\xd1\x82") {
        println!("{:<16} {:<11} {:<14} {}", "/0/100", "pci:00.0", t("мост", "bridge"), "Intel 440FX Host Bridge");
        println!("{:<16} {:<11} {:<14} {}", "/0/100/1", "pci:01.0", t("мост", "bridge"), "Intel 82371SB PIIX3 ISA Bridge");
    }
    if is_match(filter, b"display", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xbf\xd0\xbb\xd0\xb5\xd0\xb9") {
        print!("{:<16} {:<11} {:<14} {}", "/0/100/2", "fb0", t("дисплей", "display"), t("Видеоадаптер Virtio GPU (", "Virtio GPU Framebuffer ("));
        print!("{})", inv.fb_mode);
        println!();
    }
    if is_match(filter, b"network", b"\xd1\x81\xd0\xb5\xd1\x82\xd1\x8c") {
        print!("{:<16} {:<11} {:<14} {}", "/0/100/3", "eth0", t("сеть", "network"), t("Сетевой адаптер Virtio (", "Virtio Network ("));
        print!("{})", inv.net_mac);
        println!();
    }
    if is_match(filter, b"disk", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xba") {
        for i in 0..inv.disk_count {
            let d = &inv.disks[i];
            let d_name = core::str::from_utf8(&d.name[..d.name_len]).unwrap_or("vda");
            print!("{:<16} {:<11} {:<14} {}", "/0/100/4", d_name, t("диск", "disk"), t("Блочный накопитель Virtio", "Virtio Block Disk"));
            if d.size_mb > 0 {
                print!(" ({} {})", d.size_mb, t("МиБ", "MiB"));
            }
            println!();
        }
    }
    if is_match(filter, b"input", b"\xd0\xb2\xd0\xb2\xd0\xbe\xd0\xb4") {
        println!("{:<16} {:<11} {:<14} {}", "/0/100/5", "input0", t("ввод", "input"), t("Планшет абсолютного позиционирования Virtio", "Virtio Tablet Absolute Pointer"));
    }
    if is_match(filter, b"communication", b"\xd1\x81\xd0\xb2\xd1\x8f\xd0\xb7\xd1\x8c") {
        println!("{:<16} {:<11} {:<14} {}", "/0/200", "/dev/ttyS0", t("связь", "communication"), "16550A Serial Port (COM1, 0x3F8, IRQ 4)");
    }
    if is_match(filter, b"input", b"\xd0\xb2\xd0\xb2\xd0\xbe\xd0\xb4") {
        println!("{:<16} {:<11} {:<14} {}", "/0/201", "kbd0", t("ввод", "input"), "PS/2 AT Keyboard (i8042, 0x60/0x64, IRQ 1)");
    }
    if is_match(filter, b"system", b"\xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xb0") {
        println!("{:<16} {:<11} {:<14} {}", "/0/202", "/dev/rtc0", t("система", "system"), "MC146818 CMOS Real-Time Clock (0x70/0x71, IRQ 8)");
    }
}

pub fn render_tree(inv: &HwInventory, filter: Option<&[u8]>) {
    println!("{}", t("*-core", "*-core"));
    println!("     {}", t("описание: Системная плата", "description: Motherboard"));
    println!("     {}: {} ({})", t("продукт", "product"), "Wunix Standard PC", inv.cpu_vendor);
    println!("     {}", t("физический ID: 0", "physical id: 0"));

    if is_match(filter, b"memory", b"\xd0\xbf\xd0\xb0\xd0\xbc\xd1\x8f\xd1\x82\xd1\x8c") {
        println!("  *-memory");
        println!("       {}: {}", t("описание", "description"), t("Системная память RAM", "System RAM"));
        println!("       {}: {}MiB ({}MiB {})", t("размер", "size"), inv.total_ram_mb, inv.free_ram_mb, t("свободно", "free"));
        println!("       {}: 1", t("физический ID", "physical id"));
    }

    if is_match(filter, b"processor", b"\xd0\xbf\xd1\x80\xd0\xbe\xd1\x86\xd0\xb5\xd1\x81\xd1\x81\xd0\xbe\xd1\x80") {
        println!("  *-processor");
        println!("       {}: {}", t("описание", "description"), inv.cpu_brand);
        println!("       {}: cpu0", t("логическое имя", "logical name"));
        println!("       {}: {}", t("производитель", "vendor"), inv.cpu_vendor);
        println!("       {}: 2", t("физический ID", "physical id"));
    }

    if is_match(filter, b"display", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xbf\xd0\xbb\xd0\xb5\xd0\xb9") {
        println!("  *-display");
        println!("       {}: {}", t("описание", "description"), t("Видеоадаптер Virtio GPU", "Virtio GPU Display"));
        println!("       {}: fb0", t("логическое имя", "logical name"));
        println!("       {}: {}", t("видеорежим", "mode"), inv.fb_mode);
        println!("       {}: 100/2", t("физический ID", "physical id"));
    }

    if is_match(filter, b"network", b"\xd1\x81\xd0\xb5\xd1\x82\xd1\x8c") {
        println!("  *-network");
        println!("       {}: {}", t("описание", "description"), t("Сетевой Ethernet-адаптер Virtio", "Virtio Ethernet Adapter"));
        println!("       {}: eth0", t("логическое имя", "logical name"));
        println!("       {}: {}", t("аппаратный адрес (MAC)", "hardware address (MAC)"), inv.net_mac);
        println!("       {}: 100/3", t("физический ID", "physical id"));
    }

    if is_match(filter, b"disk", b"\xd0\xb4\xd0\xb8\xd1\x81\xd0\xba") {
        for i in 0..inv.disk_count {
            let d = &inv.disks[i];
            let d_name = core::str::from_utf8(&d.name[..d.name_len]).unwrap_or("vda");
            println!("  *-disk");
            println!("       {}: {}", t("описание", "description"), t("Блочный накопитель Virtio", "Virtio Block Disk"));
            println!("       {}: /dev/{}", t("логическое имя", "logical name"), d_name);
            println!("       {}: {}MiB", t("емкость", "capacity"), d.size_mb);
            println!("       {}: 100/4", t("физический ID", "physical id"));
        }
    }
}
