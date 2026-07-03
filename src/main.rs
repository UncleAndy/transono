use anyhow::Result;

use realtime_voice_translator::audio::ring_buffer::create;

fn main() -> Result<()> {

    let (mut tx, mut rx) = create(4)?;

    tx.push(10).unwrap();
    tx.push(20).unwrap();

    assert_eq!(rx.pop().unwrap(), 10);
    assert_eq!(rx.pop().unwrap(), 20);

    Ok(())
}
