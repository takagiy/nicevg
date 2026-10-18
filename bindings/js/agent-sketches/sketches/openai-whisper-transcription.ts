// OpenAI Whisper audio transcription.
// Sources: https://github.com/openai/whisper/blob/main/whisper/audio.py,
// https://github.com/openai/whisper/blob/main/whisper/transcribe.py, and
// https://github.com/openai/whisper/blob/main/whisper/decoding.py.
// Follows decoded audio through log-Mel features and per-window decoding to
// timestamped text segments, including Whisper's fallback decode path.

import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "audio", class: "box", x: 30, y: 180, label: "audio input\npath / waveform", width: 175, height: 70 },
    { id: "mel", class: "box", x: 270, y: 180, label: "load_audio + log_mel_spectrogram\nFFmpeg -> mono 16 kHz; 80-bin features", width: 255, height: 80 },
    { id: "window", class: "box", x: 600, y: 180, label: "transcribe window loop\n30 s frames; pad_or_trim to N_FRAMES", width: 240, height: 80 },
    { id: "language", class: "box", x: 900, y: 70, label: "detect_language\nfirst window only when unspecified", width: 235, height: 76 },
    { id: "decode", class: "box", x: 900, y: 250, label: "decode(mel_segment, options)\nSOT + language + task prompt", width: 235, height: 76 },
    { id: "fallback", class: "box", x: 1210, y: 250, label: "decode fallback\ntemperature schedule on compression / low logprob", width: 260, height: 84 },
    { id: "segments", class: "box", x: 1530, y: 180, label: "result[\"segments\"]\ntext, start, end, tokens, avg_logprob", width: 240, height: 80 },
  ] satisfies Node[],
  edges: [
    { from: "audio", to: "mel", label: "audio bytes / samples" },
    { from: "mel", to: "window", label: "log-Mel tensor" },
    { from: "window", to: "language", label: "first mel window" },
    { from: "language", to: "decode", label: "language token" },
    { from: "window", to: "decode", label: "mel_segment" },
    { from: "decode", to: "fallback", label: "DecodingResult: tokens, avg_logprob, compression_ratio, no_speech_prob" },
    { from: "fallback", to: "decode", label: "retry at next temperature if thresholds fail" },
    { from: "decode", to: "segments", label: "accepted tokens -> timestamped segment" },
    { from: "segments", to: "window", label: "seek next audio window until exhausted" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
