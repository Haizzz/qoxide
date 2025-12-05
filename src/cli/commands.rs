use crate::cli::output;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use qoxide::{MessageState, QoxideQueue};
use serde::Serialize;
use std::process;

pub fn open_queue(db_path: &str) -> QoxideQueue {
    QoxideQueue::builder()
        .path(db_path)
        .build()
        .unwrap_or_else(|err| {
            eprintln!("Failed to open queue: {}", err);
            process::exit(1);
        })
}

#[derive(Serialize)]
pub struct AddResult {
    pub id: i64,
}

pub fn add(db_path: &str, payload: &str, utf8: bool, json: bool) {
    let mut queue = open_queue(db_path);

    let bytes = if utf8 {
        payload.as_bytes().to_vec()
    } else {
        BASE64.decode(payload).unwrap_or_else(|err| {
            if json {
                output::print_json_error(&format!("Invalid base64: {}", err));
            } else {
                eprintln!("Error: Invalid base64: {}", err);
            }
            process::exit(1);
        })
    };

    match queue.add(bytes) {
        Ok(id) => {
            if json {
                output::print_json(AddResult { id });
            } else {
                println!("{}", id);
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to add message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct ReserveResult {
    pub id: i64,
    pub payload: String,
}

pub fn reserve(db_path: &str, utf8: bool, json: bool) {
    let mut queue = open_queue(db_path);

    match queue.reserve() {
        Ok((id, payload)) => {
            let payload_str = if utf8 {
                String::from_utf8(payload).unwrap_or_else(|_| {
                    if json {
                        output::print_json_error("Payload is not valid UTF-8");
                    } else {
                        eprintln!("Error: Payload is not valid UTF-8");
                    }
                    process::exit(1);
                })
            } else {
                BASE64.encode(&payload)
            };

            if json {
                output::print_json(ReserveResult {
                    id,
                    payload: payload_str,
                });
            } else {
                println!("{}", id);
                println!("{}", payload_str);
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to reserve message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

pub fn complete(db_path: &str, id: i64, json: bool) {
    let queue = open_queue(db_path);

    match queue.complete(id) {
        Ok(()) => {
            if json {
                output::print_json(serde_json::json!({"id": id, "status": "completed"}));
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to complete message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct FailResult {
    pub id: i64,
    pub new_state: String,
}

pub fn fail(db_path: &str, id: i64, json: bool) {
    let mut queue = open_queue(db_path);

    match queue.fail(id) {
        Ok(new_state) => {
            let state_str = match new_state {
                MessageState::Pending => "PENDING",
                MessageState::Dead => "DEAD",
                _ => "UNKNOWN",
            };

            if json {
                output::print_json(FailResult {
                    id,
                    new_state: state_str.to_string(),
                });
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to fail message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

pub fn remove(db_path: &str, id: i64, json: bool) {
    let mut queue = open_queue(db_path);

    match queue.remove(id) {
        Ok(()) => {
            if json {
                output::print_json(serde_json::json!({"id": id, "status": "removed"}));
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to remove message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct GetResult {
    pub id: i64,
    pub payload: String,
}

pub fn get(db_path: &str, id: i64, utf8: bool, json: bool) {
    let queue = open_queue(db_path);

    match queue.get(id) {
        Ok(payload) => {
            let payload_str = if utf8 {
                String::from_utf8(payload).unwrap_or_else(|_| {
                    if json {
                        output::print_json_error("Payload is not valid UTF-8");
                    } else {
                        eprintln!("Error: Payload is not valid UTF-8");
                    }
                    process::exit(1);
                })
            } else {
                BASE64.encode(&payload)
            };

            if json {
                output::print_json(GetResult {
                    id,
                    payload: payload_str,
                });
            } else {
                println!("{}", payload_str);
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to get message: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct SizeResult {
    pub total: usize,
    pub pending: usize,
    pub reserved: usize,
    pub completed: usize,
    pub dead: usize,
}

pub fn show_size(db_path: &str, json: bool) {
    let queue = open_queue(db_path);

    match queue.size() {
        Ok(size) => {
            if json {
                output::print_json(SizeResult {
                    total: size.total,
                    pending: size.pending,
                    reserved: size.reserved,
                    completed: size.completed,
                    dead: size.dead,
                });
            } else {
                println!("total {}", size.total);
                println!("pending {}", size.pending);
                println!("reserved {}", size.reserved);
                println!("completed {}", size.completed);
                println!("dead {}", size.dead);
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to get queue size: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct DeadLettersResult {
    pub ids: Vec<i64>,
    pub count: usize,
}

pub fn list_dead_letters(db_path: &str, json: bool) {
    let queue = open_queue(db_path);

    match queue.dead_letters() {
        Ok(ids) => {
            if json {
                output::print_json(DeadLettersResult {
                    ids: ids.clone(),
                    count: ids.len(),
                });
            } else {
                for id in ids {
                    println!("{}", id);
                }
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to list dead letters: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
pub struct RequeueResult {
    pub requeued: Vec<i64>,
    pub count: usize,
}

pub fn requeue_dead_letters(db_path: &str, ids: &[i64], json: bool) {
    let mut queue = open_queue(db_path);

    match queue.requeue_dead_letters(ids) {
        Ok(()) => {
            if json {
                output::print_json(RequeueResult {
                    requeued: ids.to_vec(),
                    count: ids.len(),
                });
            }
        }
        Err(err) => {
            if json {
                output::print_json_error(&format!("Failed to requeue messages: {}", err));
            } else {
                eprintln!("{}", err);
            }
            process::exit(1);
        }
    }
}
