use crate::audio::Audio;
use crate::providers::openai::translation::SessionInfo;

pub enum SessionEvent {
    SessionStarted(SessionInfo),

    /// Очередная порция аудио.
    Audio(Audio),

    /// Очередная порция текста.
    Text(String),

    /// Провайдер начал принимать/обрабатывать новый пользовательский запрос
    RequestStarted,

    /// Пользовательский запрос полностью получен (ввод завершён)
    RequestFinished,

    /// Началась генерация ответа
    ResponseStarted,

    /// Ответ полностью сформирован и передан
    ResponseFinished,
}
