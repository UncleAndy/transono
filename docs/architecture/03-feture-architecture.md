# Future Architecture

## Internal audio representation

- Internal audio format is PcmAudio
- Fixed sample rate: 48000 Hz
- Fixed sample format: f32
- Mono only
- Stereo and multichannel are represented as multiple processing lines

---

## Runtime

Audio graph consists of nodes.

Every node exposes:

- InputPort
- OutputPort

Implemented nodes:

- AudioLink
- AudioSplitter
- AudioMixer
- TranslationProcessor
- Capture
- Playback

---

## Node classes

### Transport nodes

- AudioLink
- AudioSplitter

Transport nodes never inspect audio payload.

### DSP nodes

- Translation
- Mixer
- Resampler
- Recorder
- Limiter
- AGC

DSP nodes operate on PcmAudio.

---

## TranslationLine decomposition

TranslationLine should be decomposed into:

CaptureAdapter
↓

TranslationProcessor
↓

PlaybackAdapter

---

## Future work

- internal fixed audio format
- TranslationLine decomposition
- Mixer
- Graph builder
