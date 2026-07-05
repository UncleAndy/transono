use crate::audio::Audio;

pub enum SessionEvent {
    /// Очередная порция аудио.
    Audio(Audio),

    /// Провайдер начал принимать/обрабатывать новый пользовательский запрос
    RequestStarted,

    /// Пользовательский запрос полностью получен (ввод завершён)
    RequestFinished,

    /// Началась генерация ответа
    ResponseStarted,

    /// Ответ полностью сформирован и передан
    ResponseFinished,

    Error(anyhow::Error),
}
