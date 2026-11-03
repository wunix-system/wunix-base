/// Вычисляет стандартную 16-битную контрольную сумму Интернет (RFC 1071, 1's complement)
pub fn calc_checksum(buf: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < buf.len() {
        let word = u16::from_be_bytes([buf[i], buf[i + 1]]);
        sum = sum.wrapping_add(word as u32);
        i += 2;
    }
    if i < buf.len() {
        sum = sum.wrapping_add((buf[i] as u32) << 8);
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn parse_u32(s: &[u8]) -> Option<u32> {
    let mut val = 0u32;
    if s.is_empty() {
        return None;
    }
    for &b in s {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u32;
        } else {
            return None;
        }
    }
    Some(val)
}

pub fn build_echo_packet(buf: &mut [u8], ident: u16, seq: u32, data_size: usize) {
    let total_pkt_len = 8 + data_size;
    let pkt_slice = &mut buf[..total_pkt_len];
    pkt_slice.fill(0);
    pkt_slice[0] = 8; // ICMP Type 8: Echo Request
    pkt_slice[1] = 0; // Code 0
    pkt_slice[4] = (ident >> 8) as u8;
    pkt_slice[5] = (ident & 0xFF) as u8;
    pkt_slice[6] = ((seq >> 8) & 0xFF) as u8;
    pkt_slice[7] = (seq & 0xFF) as u8;

    for p in 8..total_pkt_len {
        pkt_slice[p] = b'a' + (((p - 8) % 23) as u8);
    }

    let csum = calc_checksum(pkt_slice);
    pkt_slice[2] = (csum >> 8) as u8;
    pkt_slice[3] = (csum & 0xFF) as u8;
}
