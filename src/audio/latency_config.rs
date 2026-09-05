//! Целевая аппаратная задержка для горячего аудиопути.
//!
//! Используется, чтобы подменить дефолтный квант устройства
//! (`BufferSize::Default`, дающий ~21 мс на направление при 48 кГц) на
//! фиксированный буфер выбранного размера. Одно место правды — размер
//! легко двигать в одну сторону.

/// Целевая аппаратная задержка на направление, мс.
///
/// 480 фреймов @ 48 кГц ≈ 10 мс, 256 ≈ 5.3 мс.
pub const TARGET_LATENCY_MS: u32 = 10;

/// Число фреймов под целевую задержку для данной частоты.
///
/// Никогда не возвращает 0 — минимум один фрейм.
pub fn latency_frames(sample_rate: u32) -> u32 {
    (sample_rate * TARGET_LATENCY_MS / 1000).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_at_48k_are_480() {
        assert_eq!(latency_frames(48000), 480);
    }

    #[test]
    fn frames_at_44100_are_441() {
        assert_eq!(latency_frames(44100), 441);
    }

    #[test]
    fn never_returns_zero() {
        // Целочисленное деление может дать 0 для очень низких частот —
        // функция обязана вернуть хотя бы 1 фрейм.
        assert_eq!(latency_frames(0), 1);
        assert_eq!(latency_frames(50), 1);
        assert_eq!(latency_frames(8000), 80);
    }

    #[test]
    fn monotonic_in_sample_rate() {
        assert!(latency_frames(16000) < latency_frames(44100));
        assert!(latency_frames(44100) < latency_frames(48000));
    }

    #[test]
    fn approximates_target_latency() {
        for rate in [8000u32, 16000, 44100, 48000] {
            let frames = latency_frames(rate);
            let ms = frames as f64 / rate as f64 * 1000.0;
            assert!(
                (ms - TARGET_LATENCY_MS as f64).abs() < 1.0,
                "rate {rate}: {ms:.3} ms ~= {TARGET_LATENCY_MS} ms"
            );
        }
    }
}
