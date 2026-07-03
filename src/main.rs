mod audio;
mod openai;

use crate::openai::protocol::*;

fn main() {
    let s = SessionUpdate::new(
        "gpt-realtime",
        "You are translator",
        "alloy",
    );

    println!("{}", serde_json::to_string_pretty(&s).unwrap());
}
