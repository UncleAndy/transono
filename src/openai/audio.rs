use base64::{engine::general_purpose::STANDARD, Engine};

/// PCM16 -> Base64
#[inline]
pub fn pcm16_to_base64(samples: &[i16]) -> String {
    let bytes = unsafe {
        std::slice::from_raw_parts(
            samples.as_ptr() as *const u8,
            samples.len() * std::mem::size_of::<i16>(),
        )
    };

    STANDARD.encode(bytes)
}

/// Base64 -> PCM16
#[inline]
pub fn base64_to_pcm16(data: &str) -> anyhow::Result<Vec<i16>> {
    let bytes = STANDARD.decode(data)?;

    if bytes.len() % 2 != 0 {
        anyhow::bail!("PCM16 payload has odd byte count");
    }

    let mut pcm = Vec::with_capacity(bytes.len() / 2);

    for chunk in bytes.chunks_exact(2) {
        pcm.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(pcm)
}
