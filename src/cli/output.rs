use serde::Serialize;
use serde_json;

#[derive(Serialize)]
pub struct JsonOutput<T: Serialize> {
    pub success: bool,
    pub data: T,
}

#[derive(Serialize)]
pub struct JsonError {
    pub success: bool,
    pub error: String,
}

pub fn print_json<T: Serialize>(data: T) {
    let output = JsonOutput {
        success: true,
        data,
    };
    println!("{}", serde_json::to_string(&output).unwrap());
}

pub fn print_json_error(error: &str) {
    let output = JsonError {
        success: false,
        error: error.to_string(),
    };
    eprintln!("{}", serde_json::to_string(&output).unwrap());
}
