use anyhow::Result;

/// Любой обработчик аудиопотока.
pub trait AudioProcessor: Send {
    fn process(
        &mut self,
        input: &[i16],
        output: &mut Vec<Vec<i16>>,
    ) -> Result<()>;
}
