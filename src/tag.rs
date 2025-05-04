use std::net::IpAddr;
use std::time::SystemTime;

#[derive(Debug)]
pub struct Tagged<T> {
    pub time: SystemTime,
    pub real_ip: IpAddr,

    pub value: T,
}

impl<T> Tagged<T> {
    pub fn now(value: T, real_ip: IpAddr) -> Self {
        Self {
            value,
            real_ip,
            time: SystemTime::now(),
        }
    }
}
