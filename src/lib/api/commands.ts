/**
 * Typed wrappers over invoke(). Every call rejects with AppError (narrow
 * with isAppError) when the Rust side returns Err.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	ExportFormat,
	InputDevice,
	ModelInfo,
	ProductInfo,
	RecordingInfo,
	Settings,
	TranscribeSource,
	Transcript,
	TranscriptMeta
} from './types';

export const productInfo = () => invoke<ProductInfo>('product_info');

// models
export const listModels = () => invoke<ModelInfo[]>('list_models');
/** Resolves when the download finishes; progress streams as events. */
export const downloadModel = (modelId: string) => invoke<void>('download_model', { modelId });
export const cancelDownload = (modelId: string) => invoke<void>('cancel_download', { modelId });
export const deleteModel = (modelId: string) => invoke<void>('delete_model', { modelId });

// recording
export const listInputDevices = () => invoke<InputDevice[]>('list_input_devices');
export const startRecording = (deviceId?: string) =>
	invoke<void>('start_recording', { deviceId: deviceId ?? null });
export const stopRecording = () => invoke<RecordingInfo>('stop_recording');

// transcription
export const transcribe = (source: TranscribeSource, modelId: string, language?: string) =>
	invoke<number>('transcribe', { source, modelId, language: language ?? null });
export const cancelJob = (jobId: number) => invoke<void>('cancel_job', { jobId });

// library
export const listTranscripts = () => invoke<TranscriptMeta[]>('list_transcripts');
export const getTranscript = (id: string) => invoke<Transcript>('get_transcript', { id });
export const updateTranscript = (id: string, segmentId: number, text: string) =>
	invoke<Transcript>('update_transcript', { id, segmentId, text });
export const renameTranscript = (id: string, title: string) =>
	invoke<Transcript>('rename_transcript', { id, title });
export const deleteTranscript = (id: string) => invoke<void>('delete_transcript', { id });

// player
export const playerLoad = (id: string) => invoke<void>('player_load', { id });
export const playerPlay = () => invoke<void>('player_play');
export const playerPause = () => invoke<void>('player_pause');
export const playerSeek = (secs: number) => invoke<void>('player_seek', { secs });
export const playerPeaks = (id: string, buckets?: number) =>
	invoke<number[]>('player_peaks', { id, buckets: buckets ?? null });

// export
export const exportTranscript = (id: string, format: ExportFormat, destPath: string) =>
	invoke<void>('export_transcript', { id, format, destPath });

// settings
export const getSettings = () => invoke<Settings>('get_settings');
export const setSettings = (settings: Settings) => invoke<void>('set_settings', { settings });
