// use super::{Room, Timestamped};
// use serde::{ser::SerializeSeq, Deserialize, Serialize};
// use std::collections::HashMap;
// use std::net::IpAddr;
// use uuid::Uuid;
//
// #[derive(Deserialize, Debug)]
// pub enum Rule {
//     Exact(IpAddr),
//     NameContains(String),
// }
//
// pub struct FilteredSerializer<'a> {
//     rooms: &'a HashMap<Uuid, Timestamped<Room>>,
//     rules: &'a [Rule],
// }
//
// impl<'a> Serialize for FilteredSerializer<'a> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         if self.rules.is_empty() {
//             let mut seq = serializer.serialize_seq(Some(self.rooms.len()))?;
//             for room in self.rooms.values() {
//                 seq.serialize_element(&room.value)?;
//             }
//             seq.end()
//         } else {
//             let mut seq = serializer.serialize_seq(None)?;
//             for room in self.rooms.values() {
//                 if self.rules.iter().any(|rule| rule.allows(room)) {
//                     seq.serialize_element(&room.value)?;
//                 }
//             }
//             seq.end()
//         }
//     }
// }
//
// impl Rule {
//     fn allows(&self, room: &Timestamped<Room>) -> bool {
//         match self {
//             Rule::Exact(ip) => Some(ip) == room.value.address.as_ref(),
//             Rule::NameContains(name) => room.value.name.contains(name),
//         }
//     }
// }
