use std::collections::HashMap;
use std::net::IpAddr;

// Since we're not using any sort of formal authentication; it's probably a good idea to
// prevent people from spawning an infinite amount of rooms.
pub struct UsageTracker {
    usage_count_by_ip: HashMap<IpAddr, u16>,
    limit: u16,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            usage_count_by_ip: HashMap::new(),
            limit: 5,
        }
    }

    pub fn increase(&mut self, ip: IpAddr) -> Result<(), RateLimited> {
        let rooms_from_this_ip = self.usage_count_by_ip.entry(ip).or_insert(0);
        *rooms_from_this_ip += 1;

        if *rooms_from_this_ip > self.limit {
            *rooms_from_this_ip -= 1;
            return Err(RateLimited);
        } else {
            return Ok(());
        }
    }

    pub fn decrease(&mut self, ip: &IpAddr) {
        match self.usage_count_by_ip.get_mut(ip) {
            Some(n) if *n < 2 => {
                self.usage_count_by_ip.remove(ip);
            }
            Some(n) => *n -= 1,
            None => eprintln!("{ip} was nto registered under the rate limiter!"),
        }
    }
}

pub struct RateLimited;
