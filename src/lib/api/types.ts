/**
 * Mirrors of the Rust IPC types. Contract:
 * docs/diagrams/architecture/ipc-transcribe-sequence.md — if a shape changes
 * here it changed in src-tauri first.
 */

export interface ProductInfo {
	name: string;
	version: string;
}

// ---------------------------------------------------------------- errors

export type ErrorKind =
	| 'unknown_model'
	| 'insufficient_disk'
	| 'checksum_mismatch'
	| 'network'
	| 'model_not_found'
	| 'unsupported_media'
	| 'decode'
	| 'empty_audio'
	| 'audio_device'
	| 'capture'
	| 'playback'
	| 'transcript_not_found'
	| 'store'
	| 'engine'
	| 'cancelled'
	| 'io'
	| 'busy'
	| 'bad_request';

export interface AppError {
	kind: ErrorKind;
	message: string;
}

/** Narrow an unknown invoke() rejection to AppError. */
export function isAppError(e: unknown): e is AppError {
	return (
		typeof e === 'object' &&
		e !== null &&
		'kind' in e &&
		'message' in e &&
		typeof (e as AppError).message === 'string'
	);
}

// ---------------------------------------------------------------- models

export type ModelState =
	| { state: 'not_downloaded' }
	| { state: 'partial'; bytes: number }
	| { state: 'ready' };

export interface ModelInfo {
	id: string;
	displayName: string;
	sizeBytes: number;
	state: ModelState;
	isDefault: boolean;
}

export interface DownloadProgressEvent {
	modelId: string;
	bytes: number;
	total: number;
}

// ------------------------------------------------------------- recording

export interface InputDevice {
	id: string;
	name: string;
	isDefault: boolean;
}

export interface LevelUpdate {
	rms: number;
	peak: number;
	elapsedSecs: number;
}

export interface RecordingInfo {
	durationSecs: number;
	interrupted: boolean;
}

// ------------------------------------------------------------ transcribe

export type TranscribeSource = { type: 'file'; path: string } | { type: 'recording' };

export interface EngineSegment {
	start: number;
	end: number;
	text: string;
}

export interface SegmentEvent {
	jobId: number;
	segments: EngineSegment[];
}

export interface ProgressEvent {
	jobId: number;
	phase: 'decode' | 'transcribe';
	pct: number;
}

export interface DoneEvent {
	jobId: number;
	transcriptId: string;
}

export interface JobErrorEvent {
	jobId: number;
	error: AppError;
}

// --------------------------------------------------------------- library

export interface TranscriptSegment {
	id: number;
	start: number;
	end: number;
	text: string;
	originalText?: string;
}

export interface Transcript {
	schemaVersion: number;
	id: string;
	title: string;
	createdAt: string;
	updatedAt: string;
	sourceFilename?: string;
	duration: number;
	modelId: string;
	language: string;
	transcriptionDurationSecs?: number;
	audioRelativePath?: string;
	segments: TranscriptSegment[];
}

export interface TranscriptMeta {
	id: string;
	title: string;
	createdAt: string;
	updatedAt: string;
	duration: number;
	modelId: string;
	language: string;
	hasAudio: boolean;
}

// ---------------------------------------------------------------- player

export interface PlaybackPosition {
	secs: number;
	playing: boolean;
}

// ---------------------------------------------------------------- export

export type ExportFormat = 'txt' | 'srt' | 'vtt' | 'json';

// -------------------------------------------------------------- settings

export interface Settings {
	defaultModelId: string;
	language: string;
	inputDeviceId: string | null;
	storageDir: string | null;
	fxEnabled: boolean;
}
