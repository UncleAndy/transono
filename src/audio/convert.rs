use bytes::Bytes;

use crate::audio::{Audio, AudioEncoding, AudioFormat, EncodedAudio};
use crate::core::error::Result;

pub trait AudioCodec: Send + Sync {
    fn encoding(&self) -> AudioEncoding;

    fn encode(
        &self,
        audio: &Audio,
    ) -> Result<EncodedAudio>;

    // Отдельно передавать AudioFormat не нужно, т.к. в закодированном аудио уже
    // есть заголовки с характеристиками
    fn decode(
        &self,
        data: &EncodedAudio,
    ) -> Result<Audio>;
}

pub trait AudioConverter: Send + Sync {
        fn convert(
        &mut self,
        audio: Audio,
    ) -> Result<Audio>;
}
