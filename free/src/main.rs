#![no_std]
#![no_main]

use libwunix::{STDERR_FILENO, STDOUT_FILENO};

const NUMBER_BUFFER_SIZE: usize = 20;

#[repr(C)]
struct SysInfo {
    uptime: i64,
    loads: [u64; 3],
    totalram: u64,
    freeram: u64,
    sharedram: u64,
    bufferram: u64,
    totalswap: u64,
    freeswap: u64,
    procs: u16,
    totalhigh: u64,
    freehigh: u64,
    mem_unit: u32,
    _f: [u8; 20 - 2 * core::mem::size_of::<u64>() - core::mem::size_of::<u32>()],
}

fn print_u64(mut n: u64) {
    if n == 0 {
        libwunix::write(STDOUT_FILENO, b"0");
        return;
    }
    let mut buf = [0u8; NUMBER_BUFFER_SIZE];
    let mut i = buf.len();
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    if let Some(slice) = buf.get(i..) {
        libwunix::write(STDOUT_FILENO, slice);
    }
}

// Запрос объема оперативной памяти у ядра через sysinfo.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut info = core::mem::MaybeUninit::<SysInfo>::uninit();
    let ret = libwunix::sysinfo(info.as_mut_ptr() as *mut u8);
    if ret == 0 {
        let inf = unsafe { info.assume_init() };
        let unit = if inf.mem_unit > 0 { inf.mem_unit as u64 } else { 1 };
        let total_kb = (inf.totalram * unit) / 1024;
        let free_kb = (inf.freeram * unit) / 1024;
        let buffer_kb = (inf.bufferram * unit) / 1024;
        let used_kb = total_kb.saturating_sub(free_kb + buffer_kb);

        libwunix::write(
            STDOUT_FILENO,
            libwunix::t(
                "               всего       занято     свободно        общая  буф/кэш     доступно\nПамять:    ",
                "               total        used        free      shared  buff/cache   available\nMem:       ",
            )
            .as_bytes(),
        );
        print_u64(total_kb);
        libwunix::write(STDOUT_FILENO, b"       ");
        print_u64(used_kb);
        libwunix::write(STDOUT_FILENO, b"       ");
        print_u64(free_kb);
        libwunix::write(STDOUT_FILENO, b"           0       ");
        print_u64(buffer_kb);
        libwunix::write(STDOUT_FILENO, b"       ");
        print_u64(free_kb + buffer_kb);
        libwunix::write(STDOUT_FILENO, b"\n");
        libwunix::exit(0);
    } else {
        libwunix::write(STDERR_FILENO, b"free: cannot read memory statistics from kernel\n");
        libwunix::exit(1);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in free\n");
    libwunix::exit(1);
}
