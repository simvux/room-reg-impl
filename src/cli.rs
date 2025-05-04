use crate::Storage;
use easy_repl::{command, CommandStatus, Repl};
use std::net::IpAddr;
use std::collections::HashMap;
use uuid::Uuid;
use owo_colors::OwoColorize;

pub fn listener(rooms: Storage) {
    let status_rooms = rooms.clone();
    let usagetracker_rooms = rooms.clone();
    let add_limit_rooms = rooms.clone();
    let remove_rooms = rooms.clone();

    let mut repl = Repl::builder()
        .add(
            "exit",
            command! {
                "Exit",
                () => || Ok(CommandStatus::Quit),

            },
        )
        .add(
            "stats",
            command! {
                "Show information",
                () => || {
                    let info = status_rooms.read().unwrap();
                    
                    let users: usize = info.rooms.values().map(|room| room.value.players.len()).sum();

                    let lobbies_by_games: HashMap<&str, Vec<&str>> = info.rooms.values().fold(HashMap::new(), |mut games, room| {
                        games.entry(&room.value.game_name()).or_insert_with(Vec::new).push(&room.value.name);
                        games
                    });

                    let players_by_games: HashMap<&str, usize> = info.rooms.values().fold(HashMap::new(), |mut games, room| {
                        for member in &room.value.players {
                            *games.entry(&member.gameName).or_insert(0) += 1;
                        }
                        games
                    });

                    println!("{} ({}): {lobbies_by_games:#?}", "Game Rooms".green(), info.rooms.len().purple());
                    println!("{} ({}): {players_by_games:#?}", "Game Players".green(), users.purple());

                    Ok(CommandStatus::Done)
                },
            },
        )
        .add("usagetracker", 
            command! {
            "Show usage tracker and limiter information",
                () => || {
                    let info = usagetracker_rooms.read().unwrap();
                    println!("{}\n{}", "Usage Tracker".green(), &info.usage);
                    Ok(CommandStatus::Done)
                },
            },
            )
        .add(
            "limit",
            command! {
                "Change the room limit for an IP",
                (ip: IpAddr, limit: u16) => |ip, limit| {
                    let mut info = add_limit_rooms.write().unwrap();
                    info.add_and_apply_limit(ip,  limit);
                    println!("ok.");
                    Ok(CommandStatus::Done)
                },
            },
        )
        .add(
            "remove",
            command! {
                "Remove a room",
                (uuid: Uuid) => |uuid| {
                    let mut info = remove_rooms.write().unwrap();
                    if let Some(room) = info.remove(&uuid) {
                        eprintln!("removed room {}", room.name);
                    };
                    Ok(CommandStatus::Done)
                },
            }
        )
        .build()
        .expect("Failed to create REPL");

    if let Err(err) = repl.run() {
        eprintln!("{err}");
    }
}
