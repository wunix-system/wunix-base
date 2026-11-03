use libwunix::{close, open, read};

#[derive(Clone, Copy)]
pub struct DeviceEntry {
    pub name: [u8; 20],
    pub name_len: usize,
    pub driver: [u8; 20],
    pub driver_len: usize,
    pub irq: [u8; 8],
    pub irq_len: usize,
    pub io: [u8; 20],
    pub io_len: usize,
    pub mmio: [u8; 24],
    pub mmio_len: usize,
    pub status: &'static str,
    pub status_ru: &'static str,
}

impl DeviceEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0; 20],
            name_len: 0,
            driver: [0; 20],
            driver_len: 0,
            irq: [0; 8],
            irq_len: 0,
            io: [0; 20],
            io_len: 0,
            mmio: [0; 24],
            mmio_len: 0,
            status: "Active",
            status_ru: "Активен",
        }
    }
}

pub fn read_file<'a>(path: &[u8], buf: &'a mut [u8]) -> &'a str {
    let fd = open(path, 0, 0);
    if fd < 0 {
        return "";
    }
    let n = read(fd as u64, buf);
    close(fd as u64);
    if n > 0 {
        core::str::from_utf8(&buf[..(n as usize).min(buf.len())]).unwrap_or("")
    } else {
        ""
    }
}

pub fn collect_devices(
    ioports_text: &str,
    iomem_text: &str,
    irq_text: &str,
    devices: &mut [DeviceEntry; 16],
) -> usize {
    let mut dev_count = 0usize;

    // Парсим устройства из /proc/ioports
    for line in ioports_text.lines() {
        if line.is_empty() || dev_count >= 16 {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let range = line[..pos].trim().as_bytes();
            let drv = line[pos + 1..].trim().as_bytes();

            let entry = &mut devices[dev_count];
            let rlen = range.len().min(entry.io.len());
            entry.io[..rlen].copy_from_slice(&range[..rlen]);
            entry.io_len = rlen;

            let dlen = drv.len().min(entry.driver.len());
            entry.driver[..dlen].copy_from_slice(&drv[..dlen]);
            entry.driver_len = dlen;

            let name_str: &[u8] = match drv {
                b"pic1" | b"pic2" => b"pic",
                b"timer0" => b"timer0",
                b"i8042" => b"keyboard",
                b"rtc0" => b"rtc",
                b"serial" | b"serial (ttyS0)" => b"serial-com1",
                b"pci" => b"pci-host",
                b"virtio-blk" => b"virtio-blk",
                b"virtio-net" => b"virtio-net",
                b"virtio-input" => b"virtio-input",
                _ => drv,
            };
            let nlen = name_str.len().min(entry.name.len());
            entry.name[..nlen].copy_from_slice(&name_str[..nlen]);
            entry.name_len = nlen;

            // Сопоставляем IRQ по имени устройства
            for irq_line in irq_text.lines() {
                if let Some(col_pos) = irq_line.find(':') {
                    let num = irq_line[..col_pos].trim().as_bytes();
                    let desc = irq_line[col_pos + 1..].trim().as_bytes();
                    if desc.windows(drv.len()).any(|w| w == drv)
                        || (name_str == b"keyboard" && desc.windows(5).any(|w| w == b"i8042"))
                        || (name_str == b"serial-com1" && desc.windows(6).any(|w| w == b"serial"))
                        || (name_str == b"rtc" && desc.windows(3).any(|w| w == b"rtc"))
                        || (name_str == b"timer0" && desc.windows(5).any(|w| w == b"timer"))
                    {
                        let ilen = num.len().min(entry.irq.len());
                        entry.irq[..ilen].copy_from_slice(&num[..ilen]);
                        entry.irq_len = ilen;
                        break;
                    }
                }
            }

            entry.status = "Running";
            entry.status_ru = "В работе";
            dev_count += 1;
        }
    }

    // Добавляем устройства MMIO из /proc/iomem (например, virtio-gpu и Framebuffer)
    for line in iomem_text.lines() {
        if line.is_empty() || dev_count >= 16 {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let range = line[..pos].trim().as_bytes();
            let name_str = line[pos + 1..].trim().as_bytes();

            if name_str == b"virtio-gpu" || name_str == b"Framebuffer" {
                let entry = &mut devices[dev_count];
                let mlen = range.len().min(entry.mmio.len());
                entry.mmio[..mlen].copy_from_slice(&range[..mlen]);
                entry.mmio_len = mlen;

                let nlen = name_str.len().min(entry.name.len());
                entry.name[..nlen].copy_from_slice(&name_str[..nlen]);
                entry.name_len = nlen;

                let d_str: &[u8] = if name_str == b"virtio-gpu" { b"virtio_gpu" } else { b"limine_fb" };
                let dlen = d_str.len().min(entry.driver.len());
                entry.driver[..dlen].copy_from_slice(&d_str[..dlen]);
                entry.driver_len = dlen;

                entry.status = "Ready";
                entry.status_ru = "Готов";
                dev_count += 1;
            }
        }
    }

    dev_count
}
