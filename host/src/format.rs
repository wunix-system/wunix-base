use libwunix::{println, t, DnsError};

pub fn print_host_result(domain: &str, ip: [u8; 4]) {
    println!("{} has address {}.{}.{}.{}", domain, ip[0], ip[1], ip[2], ip[3]);
}

pub fn print_nslookup_result(server_ip: [u8; 4], domain: &str, ip: [u8; 4]) {
    println!("Server:\t\t{}.{}.{}.{}", server_ip[0], server_ip[1], server_ip[2], server_ip[3]);
    println!("Address:\t{}.{}.{}.{}#53\n", server_ip[0], server_ip[1], server_ip[2], server_ip[3]);
    println!("Non-authoritative answer:");
    println!("Name:\t{}", domain);
    println!("Address: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
}

pub fn print_error(domain: &str, err: DnsError, is_nslookup: bool) {
    if is_nslookup {
        match err {
            DnsError::NameError => {
                println!(
                    "** {}: {}",
                    t("сервер не может найти", "server can't find"),
                    domain
                );
                println!("NXDOMAIN");
            }
            DnsError::Timeout => {
                println!("{}", t(";; ошибка: превышено время ожидания ответа DNS", ";; connection timed out; no servers could be reached"));
            }
            _ => {
                println!(
                    "** {}: {}: {:?}",
                    t("ошибка разрешения", "resolution error"),
                    domain,
                    err
                );
            }
        }
    } else {
        match err {
            DnsError::NameError => {
                println!(
                    "Host {} not found: 3(NXDOMAIN)",
                    domain
                );
            }
            DnsError::Timeout => {
                println!("{}", t(";; превышено время ожидания ответа сервера DNS", ";; connection timed out; no servers could be reached"));
            }
            _ => {
                println!("Host {} not found: {:?}", domain, err);
            }
        }
    }
}
