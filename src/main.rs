use anyhow::Result;

pub mod audio;

use crate::audio::frame::FrameId;
use crate::audio::ring_buffer::FrameQueues;

fn main() -> Result<()> {

    let mut queues = FrameQueues::new(4)?;

    for id in 0..4 {
        queues.free_tx.push(id as FrameId).unwrap();
    }

    assert_eq!(queues.free_rx.pop().unwrap(), 0);
    assert_eq!(queues.free_rx.pop().unwrap(), 1);

    queues.filled_tx.push(42).unwrap();

    assert_eq!(queues.filled_rx.pop().unwrap(), 42);

    Ok(())
}
