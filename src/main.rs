use anyhow::Result;

use realtime_voice_translator::audio::{*};

fn main() -> Result<()> {
    let mut pool = frame_pool::FramePool::new(4);

    assert_eq!(pool.capacity(), 4);
    assert_eq!(pool.available(), 4);

    let a = pool.acquire()?;
    let b = pool.acquire()?;

    assert_eq!(pool.available(), 2);

    pool.get_mut(a).copy_from(&[1.0, 2.0, 3.0]);

    assert_eq!(pool.get(a).samples(), &[1.0, 2.0, 3.0]);

    pool.release(a);
    pool.release(b);

    assert_eq!(pool.available(), 4);

    Ok(())
}
