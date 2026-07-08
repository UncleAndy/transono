# Roadmap

## ✅ v0.1.0

- Core
- WebSocket transport
- Testing infrastructure
- Project philosophy

---

## ✅ v0.2.0

- [x] Provider architecture
- [x] OpenAI connection
- [x] Session commands
- [x] Provider events
- [x] First realtime interaction

---

## ✅ v0.3.0

- [x] TranslationLine
- [x] Runtime
- [x] Cancellation
- [x] AudioPipeline
- [x] End-to-end realtime audio

---

## ⏳ v0.4.0

- [ ] TranslationBridge
- [ ] Translation application

---

## ⏳ v0.5.0

### Diagnostics

- [ ] Diagnostic framework
- [ ] WavDump
- [ ] PipelineProfiler
- [ ] LatencyMonitor
- [ ] QueueMonitor
- [ ] PeakMeter
- [ ] RmsMeter

### Statistics

- [ ] TranslationLine statistics
- [ ] Capture statistics
- [ ] Playback statistics
- [ ] Pipeline statistics
- [ ] Provider statistics

### Audio metadata

- [ ] Audio packet sequence numbers
- [ ] Capture timestamps
- [ ] Pipeline latency measurement
- [ ] Jitter detection

### Testing

- [ ] DSP regression tests
- [ ] Codec regression tests
- [ ] Playback regression tests
- [ ] Golden WAV tests

### Documentation

- [ ] Audio pipeline documentation
- [ ] DSP processor guide
- [ ] Custom processor examples

### Cleanup

- [ ] PCM serialization refactoring
- [ ] PcmFormat cleanup
- [ ] Internal API cleanup

---

## ⏳ v0.6.0

### Translation

- TranslationBridge
- Streaming translation pipeline
- Multiple translation channels
- Audio routing

### Providers

- Multiple providers
- Provider failover
- Tool support
- Provider capability negotiation

### Voice Processing

- Voice Activity Detection
- Automatic Gain Control
- Noise Suppression
- Echo Cancellation

### Audio Formats

- WAV support
- Opus support
- FLAC support

### Configuration

- Configuration profiles
- Runtime configuration
- Pipeline presets

### Examples

- Voice translator
- Multi-participant translation
- Local model integration
- Hybrid provider example
