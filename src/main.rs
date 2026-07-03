mod audio;

use anyhow::Result;

use crate::audio::audio_buffer::AudioBuffer;

fn main() -> Result<()> {

    let (mut capture, mut pipeline) =
        AudioBuffer::new(4)?;

    let id = capture.acquire().unwrap();

    capture.frame_mut(id).copy_from(&[1.0,2.0,3.0]);

    capture.commit(id)?;

    let id = pipeline.receive().unwrap();

    assert_eq!(
        pipeline.frame(id).samples(),
        &[1.0,2.0,3.0]
    );

    pipeline.release(id)?;

    assert!(capture.acquire().is_some());

    Ok(())
}