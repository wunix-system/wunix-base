use libwunix::println;
use crate::time::get_time_ms;

pub struct PingStats {
    pub transmitted: u32,
    pub received: u32,
    pub min_rtt: u64,
    pub max_rtt: u64,
    pub sum_rtt: u64,
    pub start_time: u64,
}

impl PingStats {
    pub fn new() -> Self {
        Self {
            transmitted: 0,
            received: 0,
            min_rtt: u64::MAX,
            max_rtt: 0,
            sum_rtt: 0,
            start_time: get_time_ms(),
        }
    }

    pub fn record_transmit(&mut self) {
        self.transmitted += 1;
    }

    pub fn record_reply(&mut self, rtt: u64) {
        self.received += 1;
        self.min_rtt = self.min_rtt.min(rtt);
        self.max_rtt = self.max_rtt.max(rtt);
        self.sum_rtt += rtt;
    }

    pub fn print_summary(&self, ip_s: &str) {
        let lost = self.transmitted.saturating_sub(self.received);
        let loss_pct = if self.transmitted > 0 {
            (lost * 100) / self.transmitted
        } else {
            100
        };
        let total_duration = get_time_ms().saturating_sub(self.start_time);

        println!();
        println!("--- {} ping statistics ---", ip_s);
        println!(
            "{} packets transmitted, {} received, {}% packet loss, time {}ms",
            self.transmitted, self.received, loss_pct, total_duration
        );
        if self.received > 0 {
            let avg_rtt = self.sum_rtt / (self.received as u64);
            let min_print = if self.min_rtt == u64::MAX { 0 } else { self.min_rtt };
            println!(
                "rtt min/avg/max/mdev = {}/{}/{}/0 ms",
                min_print, avg_rtt, self.max_rtt
            );
        }
    }
}
