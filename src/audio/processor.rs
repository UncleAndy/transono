use anyhow::Result;

/// Любой обработчик аудиопотока.
pub trait AudioProcessor: Send {
    /// Отправляет очередную порцию аудио процессору.
    fn push_audio(&mut self, input: &[i16]) -> Result<()>;

    /// Возвращает очередной готовый аудиочанк.
    fn poll_audio(&mut self) -> Result<Option<Vec<i16>>>;
}
