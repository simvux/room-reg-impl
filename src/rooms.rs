use super::Tagged;
use super::UsageTracker;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct Room {
    #[serde(default = "String::new")]
    pub externalGuid: String,
    #[serde(default = "String::new")]
    pub id: String,
    pub address: Option<IpAddr>,
    pub name: String,
    #[serde(default = "String::new")]
    description: String,
    #[serde(default = "String::new")]
    owner: String,
    port: u16,
    preferredGameName: String,
    preferredGameId: u64,
    maxPlayers: u32,
    netVersion: u32,
    hasPassword: bool,
    #[serde(default = "Vec::new")]
    pub players: Vec<Member>,
}

impl Room {
    pub fn game_name(&self) -> &str {
        &self.preferredGameName
    }
}

pub struct Rooms {
    pub rooms: HashMap<Uuid, Tagged<Room>>,
    pub usage: UsageTracker,
}

impl Serialize for Rooms {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.rooms.len()))?;
        for room in self.rooms.values() {
            seq.serialize_element(&room.value)?;
        }
        seq.end()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
#[rustfmt::skip]
pub struct Member {
    #[serde(default = "String::new")] pub nickname: String,
    #[serde(default = "String::new")] pub username: String,
    #[serde(default = "String::new")] pub gameName: String,
    #[serde(default = "String::new")] pub avatarUrl: String,
    pub gameId: u64,
}

impl Rooms {
    pub fn new(user_limits: HashMap<IpAddr, u16>) -> Self {
        Self {
            rooms: HashMap::new(),
            usage: UsageTracker::new(user_limits),
        }
    }

    pub fn add_and_apply_limit(&mut self, ip: IpAddr, limit: u16) {
        self.usage.limit(ip, limit);

        let mut roomcount = self.rooms.len() as u16;
        self.rooms.retain(|_uuid, room| {
            if room.real_ip == ip && roomcount > limit {
                self.usage.decrease(&ip);
                roomcount -= 1;
                false
            } else {
                true
            }
        });
    }

    pub fn remove(&mut self, uuid: &Uuid) -> Option<Room> {
        let Some(room) = self.rooms.remove(uuid) else {
            eprintln!("No room goes by uuid: {uuid}");
            return None;
        };

        self.usage.decrease(&room.real_ip);

        Some(room.value)
    }

    pub fn remove_timed_out_lobbies(&mut self, timeout: Duration) {
        let now = SystemTime::now();
        self.rooms.retain(|_, room| {
            let keep = now
                .duration_since(room.time)
                .map(|dur| dur < timeout)
                .unwrap_or(true);

            if !keep {
                println!("timing out room {}", room.value.name);
                self.usage.decrease(&room.real_ip);
            }

            keep
        })
    }
}
