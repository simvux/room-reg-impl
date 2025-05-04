use owo_colors::OwoColorize;
use rocket::http::{ContentType, Status};
use rocket::serde::json::{json, Json, Value};
use rocket::{delete, get, launch, post, routes, State};
use rocket_client_addr::ClientRealAddr;
use serde::Deserialize;
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
    user_limits: HashMap<IpAddr, u16>,
}

mod cli;
mod limit;
mod tag;
use limit::UsageTracker;
use tag::Tagged;
mod fake;
mod filter;
mod rooms;
use rooms::{Member, Room, Rooms};

type Storage = Arc<RwLock<Rooms>>;

#[launch]
fn rocket() -> _ {
    let config: Config = {
        let file = std::fs::File::open("config.ron").unwrap();
        ron::de::from_reader(file).unwrap()
    };
    let timeout_seconds = config.timeout_seconds;

    let roomref = Arc::new(RwLock::new(Rooms::new(config.user_limits)));

    // Periodically remove rooms that haven't refreshed themselves
    let rr = roomref.clone();
    std::thread::spawn(move || loop {
        let timeout = Duration::from_secs(timeout_seconds);
        std::thread::sleep(timeout);

        if let Ok(mut rooms) = rr.write() {
            rooms.remove_timed_out_lobbies(timeout);
        }
    });

    // Command-line interface
    let rr = roomref.clone();
    std::thread::spawn(move || cli::listener(rr));

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
                post_profile,
                silence_telemetry,
                silence_jwt_post,
                silence_jst_empty_post,
                ok_for_token_retrieval,
                ok_for_pkey_retrieval
            ],
        )
}

#[post("/profile", data = "<body>")]
fn post_profile(body: String) -> Value {
    println!("{body}");
    json!({})
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
    let mut room = body.into_inner();
    let mut info = shared.write().unwrap();

    info.usage.increase(remote_addr.ip).map_err(|_| {
        println!("\"{}\" was block by usage limits", &room.name);
        Status::TooManyRequests
    })?;

    println!("{} \"{}\"", "Registering".green(), &room.name);

    let uuid = Uuid::new_v4();

    if room.address.is_none() {
        room.address = Some(remote_addr.ip);
    }

    if info
        .rooms
        .insert(uuid, Tagged::now(room.clone(), remote_addr.ip))
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
        info.usage.decrease(&room.real_ip);
    }
}

#[post("/jwt/external/<_token>", data = "<_body>")]
fn silence_jwt_post(_token: String, _body: String) -> (ContentType, &'static str) {
    (ContentType::Plain, "")
}

#[post("/jwt/external", data = "<_body>")]
fn silence_jst_empty_post(_body: Vec<u8>) -> (ContentType, &'static str) {
    (ContentType::Plain, "")
}

#[post("/telemetry")]
fn silence_telemetry() -> (ContentType, &'static str) {
    (ContentType::Plain, "")
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
