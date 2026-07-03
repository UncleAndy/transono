mod audio;
mod openai;

use crate::openai::events::ServerEvent;

fn main() {
    let json = r#"
    {
        "type":"response.audio.done"
    }
    "#;

    let event: ServerEvent = serde_json::from_str(json).unwrap();

    println!("{event:#?}");
}
