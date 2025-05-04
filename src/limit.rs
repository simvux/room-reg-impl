use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;

// Since we're not using any sort of formal authentication; it's probably a good idea to
// prevent people from spawning an infinite amount of rooms.
pub struct UsageTracker {
    usage_count_by_ip: HashMap<IpAddr, u16>,
    user_limits: HashMap<IpAddr, u16>,
    limit: u16,
}

impl UsageTracker {
    pub fn new(user_limits: HashMap<IpAddr, u16>) -> Self {
        Self {
            usage_count_by_ip: HashMap::new(),
            user_limits,
            limit: 5,
        }
    }

    pub fn increase(&mut self, ip: IpAddr) -> Result<(), RateLimited> {
        let rooms_from_this_ip = self.usage_count_by_ip.entry(ip).or_insert(0);
        *rooms_from_this_ip += 1;

        let limit = self.user_limits.get(&ip).copied().unwrap_or(self.limit);

        if *rooms_from_this_ip > limit {
            *rooms_from_this_ip -= 1;
            return Err(RateLimited);
        } else {
            return Ok(());
        }
    }

    pub fn decrease(&mut self, ip: &IpAddr) {
        self.decrease_by(ip, 1);
    }

    pub fn decrease_by(&mut self, ip: &IpAddr, by: u16) {
        match self.usage_count_by_ip.get_mut(ip) {
            Some(n) => match n.checked_sub(by) {
                None | Some(0) => {
                    self.usage_count_by_ip.remove(ip);
                }
                Some(new) => *n = new,
            },
            None => eprintln!("{ip} was nto registered under the rate limiter!"),
        }
    }

    pub fn limit(&mut self, ip: IpAddr, limit: u16) {
        if limit == self.limit {
            // Keep the hashmap clean when defaulting limits
            self.user_limits.remove(&ip);
        } else {
            self.user_limits.insert(ip, limit);
        }
    }
}

pub struct RateLimited;

impl fmt::Display for UsageTracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "default limit is {} rooms", self.limit.purple())?;
        for (ip, limit) in &self.user_limits {
            writeln!(f, "{ip} is limited to {} rooms", limit.purple())?;
        }

        for (ip, usage) in &self.usage_count_by_ip {
            writeln!(f, "{ip} has {} room(s)", usage.purple())?;
        }

        Ok(())
    }
}
