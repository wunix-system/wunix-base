use libwunix::{clock_gettime, nanosleep};

#[derive(Default)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

pub fn get_time_ms() -> u64 {
    let mut ts = Timespec::default();
    let ptr = &mut ts as *mut Timespec as *mut u8;
    clock_gettime(1 /* CLOCK_MONOTONIC */, ptr);
    (ts.tv_sec as u64) * 1000 + (ts.tv_nsec as u64) / 1_000_000
}

pub fn sleep_ms(ms: u64) {
    let req = Timespec {
        tv_sec: (ms / 1000) as i64,
        tv_nsec: ((ms % 1000) * 1_000_000) as i64,
    };
    let mut rem = Timespec::default();
    nanosleep(&req as *const Timespec as *const u8, &mut rem as *mut Timespec as *mut u8);
}
