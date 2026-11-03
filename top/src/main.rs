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

// Интерактивный монитор системных процессов и нагрузки.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut info = core::mem::MaybeUninit::<SysInfo>::uninit();
    let ret = libwunix::sysinfo(info.as_mut_ptr() as *mut u8);
    if ret == 0 {
        let inf = unsafe { info.assume_init() };
        let uptime_val = if inf.uptime >= 0 { inf.uptime as u64 } else { 0 };
        libwunix::write(
            STDOUT_FILENO,
            libwunix::t(
                "\x1b[2J\x1b[Htop - время работы: ",
                "\x1b[2J\x1b[Htop - uptime: ",
            )
            .as_bytes(),
        );
        print_u64(uptime_val);
        libwunix::write(
            STDOUT_FILENO,
            libwunix::t(" с, процессов: ", "s, tasks: ").as_bytes(),
        );
        print_u64(inf.procs as u64);
        libwunix::write(
            STDOUT_FILENO,
            libwunix::t(
                " всего\n\n  PID ПОЛЬЗ     PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ КОМАНДА\n",
                " total\n\n  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND\n",
            )
            .as_bytes(),
        );
        libwunix::exit(0);
    } else {
        libwunix::write(STDERR_FILENO, b"top: sysinfo syscall failed\n");
        libwunix::exit(1);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in top\n");
    libwunix::exit(1);
}
