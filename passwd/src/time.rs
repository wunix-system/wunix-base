use libwunix::{getpid, sysinfo};

pub fn get_seed() -> u64 {
    let mut info_buf = [0u8; 128];
    let _ = sysinfo(info_buf.as_mut_ptr());
    let uptime = u64::from_ne_bytes([
        info_buf[0], info_buf[1], info_buf[2], info_buf[3],
        info_buf[4], info_buf[5], info_buf[6], info_buf[7],
    ]);
    let pid = getpid();
    uptime.wrapping_mul(1000003).wrapping_add(pid).wrapping_add(0x9e3779b97f4a7c15)
}
