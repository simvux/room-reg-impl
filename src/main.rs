use rocket::http::{ContentType, Status};
use rocket::serde::json::{json, Json, Value};
use rocket::{delete, get, launch, post, routes, State};
use rocket_client_addr::ClientRealAddr;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Config {
    port: u16,
    timeout_seconds: u64,
    // filters: HashMap<String, Vec<filter::Rule>>,
}

mod limit;
use limit::UsageTracker;
mod fake;
mod filter;

type Storage = Arc<RwLock<Rooms>>;

struct Rooms {
    rooms: HashMap<Uuid, Timestamped<Room>>,
    usage: UsageTracker,
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

impl Rooms {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            usage: UsageTracker::new(),
        }
    }
}

pub struct Timestamped<T> {
    time: SystemTime,
    value: T,
}

impl<T> Timestamped<T> {
    pub fn now(value: T) -> Self {
        Self {
            value,
            time: SystemTime::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
struct Room {
    #[serde(default = "String::new")]
    externalGuid: String,
    #[serde(default = "String::new")]
    id: String,
    address: Option<IpAddr>,
    name: String,
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
    players: Vec<Member>,
}

#[derive(Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
#[rustfmt::skip]
struct Member {
    #[serde(default = "String::new")] nickname: String,
    #[serde(default = "String::new")] username: String,
    #[serde(default = "String::new")] gameName: String,
    #[serde(default = "String::new")] avatarUrl: String,
    gameId: u64,
}

#[launch]
fn rocket() -> _ {
    let config: Config = {
        let file = std::fs::File::open("config.ron").unwrap();
        ron::de::from_reader(file).unwrap()
    };
    let timeout_seconds = config.timeout_seconds;

    let roomref = Arc::new(RwLock::new(Rooms::new()));

    // Periodically remove rooms that haven't refreshed themselves
    let rr = roomref.clone();
    std::thread::spawn(move || loop {
        let timeout = Duration::from_secs(timeout_seconds);
        std::thread::sleep(timeout);

        if let Ok(mut rooms) = rr.write() {
            rooms.remove_timed_out_lobbies(timeout);
        }
    });

    rocket::build()
        .configure(rocket::Config::figment().merge(("port", config.port)))
        .manage(roomref)
        .mount(
            "/",
            routes![
                get_lobbies,
                register_lobby,
                update_lobby,
                delete_lobby,
                get_profile,
                ok_for_token_retrieval,
                ok_for_pkey_retrieval
            ],
        )
}

// Client refuses to allow a token if it hasn't been verified.
#[get("/profile")]
fn get_profile() -> Value {
    json!({})
}

#[get("/lobby")]
fn get_lobbies(shared: &State<Storage>) -> Value {
    let rooms = shared.read().unwrap();
    json!({ "rooms":  serde_json::to_value(&*rooms).unwrap()})
}

// Set up a new lobby and return it's ID.
//
// Servers may then use this ID to authorize updates for that lobby.
#[post("/lobby", data = "<body>")]
fn register_lobby(
    remote_addr: &ClientRealAddr,
    body: Json<Room>,
    shared: &State<Storage>,
) -> Result<Json<Room>, Status> {
    let remote_addr = remote_addr.ip;

    let mut room = body.into_inner();

    let mut info = shared.write().unwrap();

    info.usage
        .increase(remote_addr)
        .map_err(|_| Status::TooManyRequests)?;

    let uuid = Uuid::new_v4();

    assert!(room.externalGuid.is_empty());
    assert!(room.id.is_empty());
    if room.address.is_none() {
        room.address = Some(remote_addr);
    }

    if info
        .rooms
        .insert(uuid, Timestamped::now(room.clone()))
        .is_some()
    {
        eprintln!("UUID conflict");
        return Err(Status::InternalServerError);
    };

    room.externalGuid = uuid.to_string();
    room.id = uuid.to_string();

    Ok(Json(room))
}

#[derive(Deserialize)]
struct LobbyUpdate {
    players: Vec<Member>,
}

// Update a lobby's information and reset the timeout timestamp
#[post("/lobby/<id>", data = "<body>")]
fn update_lobby(
    id: &str,
    body: Json<LobbyUpdate>,
    shared: &State<Storage>,
) -> (ContentType, Status) {
    let uuid = Uuid::parse_str(&id).unwrap();

    let mut info = shared.write().unwrap();

    let Some(room) = info.rooms.get_mut(&uuid) else {
        return (ContentType::JSON, Status::NotFound);
    };

    room.time = SystemTime::now();
    room.value.players = body.into_inner().players;

    (ContentType::JSON, Status::Ok)
}

#[delete("/lobby/<id>")]
fn delete_lobby(id: String, shared: &State<Storage>) {
    let uuid = Uuid::parse_str(&id).unwrap();
    let mut info = shared.write().unwrap();

    if let Some(room) = info.rooms.remove(&uuid) {
        if let Some(addr) = &room.value.address {
            info.usage.decrease(addr);
        }
    }
}

#[get("/jwt/external/key.pem")]
fn ok_for_pkey_retrieval() -> (ContentType, &'static str) {
    (ContentType::Plain, fake::PUB_CERTIFICATE_KEY)
}
// The previous implementation used the wrong `ContentType` by mistake.
//
// The clients now unfortunately rely on this bug, so: we need to replicate the mistakes.
#[post("/jwt/internal", data = "<_body>")]
fn ok_for_token_retrieval(_body: String) -> (ContentType, &'static str) {
    (ContentType::HTML, fake::JWT_TOKEN)
}

impl Rooms {
    pub fn remove_timed_out_lobbies(&mut self, timeout: Duration) {
        let now = SystemTime::now();
        self.rooms.retain(|_, room| {
            let keep = now
                .duration_since(room.time)
                .map(|dur| dur < timeout)
                .unwrap_or(true);

            if !keep {
                println!("timing out room {}", room.value.name);

                if let Some(addr) = &room.value.address {
                    self.usage.decrease(addr);
                }
            }

            keep
        })
    }
}
