#![no_std]
#![no_main]

use libwunix::drv::usbfs::{UsbfsDevice, UsbfsTransport};
use libwunix::{close, exit, open, println, read, t, O_RDONLY};

fn cstr_to_slice(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

fn format_3digits(n: u8, out: &mut [u8; 3]) {
    out[0] = b'0' + (n / 100);
    out[1] = b'0' + ((n / 10) % 10);
    out[2] = b'0' + (n % 10);
}

fn query_usbfs_node(bus: u8, devnum: u8) -> Option<libwunix::drv::abi::usbfs::UsbDeviceInfo> {
    let mut path = *b"/dev/bus/usb/000/000\0";
    let mut b_str = [0u8; 3];
    let mut d_str = [0u8; 3];
    format_3digits(bus, &mut b_str);
    format_3digits(devnum, &mut d_str);
    path[13..16].copy_from_slice(&b_str);
    path[17..20].copy_from_slice(&d_str);
    let dev = UsbfsDevice::open(&path).ok()?;
    dev.device_info().ok()
}

fn parse_u8(s: &str) -> u8 {
    let mut v = 0u16;
    for b in s.bytes() {
        if (b'0'..=b'9').contains(&b) {
            v = v * 10 + (b - b'0') as u16;
        } else {
            break;
        }
    }
    v.min(255) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    libwunix::init(argc, argv);

    let mut verbose = false;
    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-v" || arg == b"--verbose" {
            verbose = true;
        } else if arg == b"-h" || arg == b"--help" {
            println!(
                "{}: lsusb [-v]",
                t("Использование", "Usage")
            );
            exit(0);
        }
        i += 1;
    }

    let fd = open(b"/proc/bus/usb/devices\0", O_RDONLY as u64, 0);
    if fd < 0 {
        println!(
            "lsusb: {}",
            t("подсистема USB (/proc/bus/usb/devices) недоступна", "USB subsystem unavailable")
        );
        exit(1);
    }

    let mut buf = [0u8; 4096];
    let mut total = 0usize;
    loop {
        let n = read(fd as u64, &mut buf[total..]) as isize;
        if n <= 0 {
            break;
        }
        total += n as usize;
        if total >= buf.len() {
            break;
        }
    }
    close(fd as u64);

    let text = core::str::from_utf8(&buf[..total]).unwrap_or("");
    let mut count = 0usize;
    let mut cur_bus = 0u8;
    let mut cur_dev = 0u8;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("T:  Bus=") {
            cur_bus = parse_u8(rest);
            if let Some(d_part) = rest.split("Dev#=").nth(1) {
                cur_dev = parse_u8(d_part.trim_start());
            }
        } else if let Some(cur_vid_pid) = line.strip_prefix("P:  Vendor=") {
            count += 1;
            println!(
                "Bus {:03} Device {:03}: ID {}",
                cur_bus, cur_dev, cur_vid_pid
            );
            if verbose {
                if let Some(info) = query_usbfs_node(cur_bus, cur_dev) {
                    println!(
                        "  [usbfs] class={:#04x} sub={:#04x} proto={:#04x} mps={} bulk_in={:#04x} bulk_out={:#04x} intr_in={:#04x}",
                        info.device_class,
                        info.device_subclass,
                        info.device_protocol,
                        info.max_packet_size,
                        info.ep_bulk_in,
                        info.ep_bulk_out,
                        info.ep_intr_in
                    );
                }
            }
        }
    }

    if count == 0 {
        println!(
            "{}",
            t("USB устройства не обнаружены", "No USB devices detected")
        );
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
