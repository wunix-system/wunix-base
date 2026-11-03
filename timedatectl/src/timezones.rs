use libwunix::println;

#[allow(dead_code)]
pub struct TimezoneEntry {
    pub name: &'static str,
    pub offset_secs: i32,
    pub abbr: &'static str,
}

pub const TIMEZONES: &[TimezoneEntry] = &[
    TimezoneEntry { name: "Africa/Cairo", offset_secs: 7200, abbr: "EET" },
    TimezoneEntry { name: "America/Anchorage", offset_secs: -32400, abbr: "AKST" },
    TimezoneEntry { name: "America/Chicago", offset_secs: -21600, abbr: "CST" },
    TimezoneEntry { name: "America/Denver", offset_secs: -25200, abbr: "MST" },
    TimezoneEntry { name: "America/Los_Angeles", offset_secs: -28800, abbr: "PST" },
    TimezoneEntry { name: "America/New_York", offset_secs: -18000, abbr: "EST" },
    TimezoneEntry { name: "America/Sao_Paulo", offset_secs: -10800, abbr: "BRT" },
    TimezoneEntry { name: "Asia/Almaty", offset_secs: 18000, abbr: "+05" },
    TimezoneEntry { name: "Asia/Bangkok", offset_secs: 25200, abbr: "+07" },
    TimezoneEntry { name: "Asia/Dubai", offset_secs: 14400, abbr: "+04" },
    TimezoneEntry { name: "Asia/Hong_Kong", offset_secs: 28800, abbr: "HKT" },
    TimezoneEntry { name: "Asia/Novosibirsk", offset_secs: 25200, abbr: "+07" },
    TimezoneEntry { name: "Asia/Shanghai", offset_secs: 28800, abbr: "CST" },
    TimezoneEntry { name: "Asia/Tokyo", offset_secs: 32400, abbr: "JST" },
    TimezoneEntry { name: "Asia/Vladivostok", offset_secs: 36000, abbr: "+10" },
    TimezoneEntry { name: "Asia/Yekaterinburg", offset_secs: 18000, abbr: "YEKT" },
    TimezoneEntry { name: "Australia/Sydney", offset_secs: 36000, abbr: "AEST" },
    TimezoneEntry { name: "Europe/Berlin", offset_secs: 3600, abbr: "CET" },
    TimezoneEntry { name: "Europe/Kaliningrad", offset_secs: 7200, abbr: "EET" },
    TimezoneEntry { name: "Europe/Kyiv", offset_secs: 7200, abbr: "EET" },
    TimezoneEntry { name: "Europe/London", offset_secs: 0, abbr: "GMT" },
    TimezoneEntry { name: "Europe/Minsk", offset_secs: 10800, abbr: "MSK" },
    TimezoneEntry { name: "Europe/Moscow", offset_secs: 10800, abbr: "MSK" },
    TimezoneEntry { name: "Europe/Paris", offset_secs: 3600, abbr: "CET" },
    TimezoneEntry { name: "Europe/Rome", offset_secs: 3600, abbr: "CET" },
    TimezoneEntry { name: "Europe/Samara", offset_secs: 14400, abbr: "SAMT" },
    TimezoneEntry { name: "UTC", offset_secs: 0, abbr: "UTC" },
];

pub fn list_timezones() {
    for tz in TIMEZONES {
        println!("{}", tz.name);
    }
}

pub fn find_timezone(name: &str) -> Option<&'static TimezoneEntry> {
    for tz in TIMEZONES {
        if tz.name == name {
            return Some(tz);
        }
    }
    None
}
