use anyhow::Result;

/// Временный интерфейс ресемплера.
///
/// Пока выполняет только преобразование формата.
/// Реальная реализация на rubato будет подставлена без изменения
/// остального проекта.
pub struct Resampler;

impl Resampler {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// 48 kHz f32 -> 24 kHz i16
    pub fn capture_to_openai(
        &mut self,
        input: &[f32],
        output: &mut Vec<i16>,
    ) -> Result<()> {
        output.clear();

        // Временный вариант: простая децимация 2:1.
        // Будет заменён на rubato.
        for sample in input.iter().step_by(2) {
            let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            output.push(s);
        }

        Ok(())
    }

    /// 24 kHz i16 -> 48 kHz f32
    pub fn openai_to_playback(
        &mut self,
        input: &[i16],
        output: &mut Vec<f32>,
    ) -> Result<()> {
        output.clear();

        // Временный вариант: удвоение сэмплов.
        // Будет заменён на rubato.
        for &sample in input {
            let s = sample as f32 / i16::MAX as f32;
            output.push(s);
            output.push(s);
        }

        Ok(())
    }
}
