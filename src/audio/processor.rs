use anyhow::Result;

/// Любой обработчик аудиопотока.
pub trait AudioProcessor: Send {
    /// Принимает PCM16 24 kHz.
    ///
    /// Может вернуть:
    /// - пустой вектор (ответа пока нет);
    /// - один или несколько аудиофрагментов.
    fn process(
        &mut self,
        input: &[i16],
    ) -> Result<Vec<Vec<i16>>>;
}
