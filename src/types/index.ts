export type Flow = "render" | "capture";
export type Lang = "en" | "ar";
export type ThemeName = "dark" | "light" | "system" | "oled";

export interface AppError {
  code: "Invalid" | "NotFound" | "Unsupported" | "Audio" | "Io" | "Internal";
  message: string;
}

export interface DeviceInfo {
  id: string;
  name: string;
  flow: Flow;
  state: string;
  isDefault: boolean;
  isDefaultCommunications: boolean;
  formFactor: string;
  sampleRate: number;
  channels: number;
  bits: number;
  volume: number;
  muted: boolean;
  isVirtual: boolean;
}

export interface SessionInfo {
  id: string;
  pid: number;
  name: string;
  exeName: string;
  exePath: string;
  state: string;
  isSystem: boolean;
  volume: number;
  muted: boolean;
  peak: number;
  deviceId: string;
}

export interface MeterReading {
  peak: number;
  rms: number;
  clipped: boolean;
  clipCount: number;
}

export interface MonitorControls {
  inputGainDb: number;
  outputGainDb: number;
  monitorGainDb: number;
  monitorMute: boolean;
  processed: boolean;
}

export interface LatencyInfo {
  captureMs: number;
  ringMs: number;
  dspMs: number;
  renderMs: number;
  totalMs: number;
  method: string;
}

export interface EngineSnapshot {
  running: boolean;
  error: string | null;
  inputDevice: string | null;
  outputDevice: string | null;
  micIn: MeterReading;
  dspIn: MeterReading;
  dspOut: MeterReading;
  monitorOut: MeterReading;
  waveIn: number[];
  waveOut: number[];
  latency: LatencyInfo;
  underruns: number;
  overruns: number;
  driftCorrections: number;
  feedbackSuspected: boolean;
  controls: MonitorControls;
  framesRendered: number;
}

export interface EqBand {
  kind: "lowShelf" | "peak" | "highShelf";
  freq: number;
  gainDb: number;
  q: number;
}

export interface VoiceParams {
  inputGainDb: number;
  outputGainDb: number;
  mix: number;
  noiseSuppression: { enabled: boolean; strength: number };
  gate: { enabled: boolean; thresholdDb: number; attackMs: number; holdMs: number; releaseMs: number };
  filters: { highpassEnabled: boolean; highpassHz: number; lowpassEnabled: boolean; lowpassHz: number };
  eq: { enabled: boolean; bands: EqBand[] };
  pitch: { enabled: boolean; semitones: number; cents: number };
  formant: { enabled: boolean; shift: number };
  effects: {
    ringEnabled: boolean; ringHz: number; ringMix: number;
    saturationEnabled: boolean; saturationDrive: number; saturationMix: number;
    vibratoEnabled: boolean; vibratoRateHz: number; vibratoDepth: number;
  };
  delay: { enabled: boolean; timeMs: number; feedback: number; mix: number };
  reverb: { enabled: boolean; roomSize: number; decay: number; mix: number };
  compressor: { enabled: boolean; thresholdDb: number; ratio: number; attackMs: number; releaseMs: number; makeupDb: number };
  limiter: { enabled: boolean; ceilingDb: number };
}

export interface Preset {
  id: string;
  name: string;
  description: string;
  params: VoiceParams;
}

export interface VoiceState {
  params: VoiceParams;
  presetId: string | null;
  controls: MonitorControls;
}

export interface Settings {
  language: Lang;
  theme: ThemeName;
  startWithWindows: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  onboardingDone: boolean;
  reduceMotion: boolean;
  meterRateHz: number;
  monitorInputDevice?: string | null;
  monitorOutputDevice?: string | null;
  favorites: string[];
  hotkeys: Record<string, string>;
  activeProfile?: string | null;
  showAdvanced: boolean;
  feedbackWarning: boolean;
}

export interface Profile {
  id: string;
  name: string;
  description: string;
  voice: VoiceParams;
  voicePreset?: string | null;
  masterVolume?: number | null;
  micGainDb: number;
  micMuted?: boolean | null;
  appVolumes: Record<string, number>;
  createdMs: number;
}

export interface Bootstrap {
  settings: Settings;
  profiles: Profile[];
  presets: Preset[];
  defaultParams: VoiceParams;
  voice: VoiceState;
  hotkeyActions: [string, string][];
  version: string;
}

export interface EndpointLevel {
  deviceId: string | null;
  peak: number;
  volume: number;
  muted: boolean;
}

export interface RecorderStatus {
  recording: boolean;
  elapsedMs: number;
  targetMs: number;
  level: number;
  error: string | null;
}

export interface PlayerStatus {
  playing: boolean;
  positionMs: number;
  totalMs: number;
}

export interface Telemetry {
  master: EndpointLevel | null;
  mic: EndpointLevel | null;
  monitor: EngineSnapshot | null;
  recorder: RecorderStatus | null;
  player: PlayerStatus | null;
  hasTake: boolean;
  uptimeS: number;
}

export interface TakeInfo {
  durationMs: number;
  original: number[];
  processed?: number[] | null;
  processedKey?: string | null;
}

export interface VirtualMicStatus {
  state: "detected" | "notDetected";
  renderEndpoints: DeviceInfo[];
  captureEndpoints: DeviceInfo[];
  message: string;
}

export interface Diagnostics {
  version: string;
  os: string;
  uptimeS: number;
  dataDir: string;
  logDir: string;
  renderDevices: number;
  captureDevices: number;
  defaultRender: string | null;
  defaultCapture: string | null;
  sessionCount: number;
  routingSupported: boolean;
  monitorRunning: boolean;
  monitorError: string | null;
  virtualMic: string;
  deviceChangeEvents: number;
  notes: string[];
}

export interface HotkeyResult {
  results: { action: string; accelerator: string; registered: boolean; error?: string | null }[];
  conflicts: [string, string[]][];
}

export interface HotkeyActionEvent {
  action: string;
  message: string;
}
