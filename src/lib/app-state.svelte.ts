/**
 * Central UI state (Svelte 5 runes). One instance app-wide; components read
 * fields directly and call the mutation helpers.
 *
 * Outside the Tauri webview (plain-vite dev in a browser) every invoke()
 * rejects — load() degrades to empty data so the shell stays inspectable.
 */

import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import * as api from './api';
import type {
	AppError,
	EngineSegment,
	ModelInfo,
	Settings,
	Transcript,
	TranscribeSource,
	TranscriptMeta
} from './api';

export type AppPhase =
	| { kind: 'idle' }
	| { kind: 'recording'; elapsedSecs: number }
	| { kind: 'transcribing'; jobId: number; pct: number; phase: 'decode' | 'transcribe'; modelId: string };

export interface ModelSheetState {
	modelId: string;
	sizeBytes: number;
	bytes: number;
	total: number;
	bytesPerSec: number;
	/** The transcription to run once the download verifies. */
	pendingSource: TranscribeSource;
	verified: boolean;
	error: AppError | null;
}

export const IMPORT_EXTENSIONS = ['mp3', 'm4a', 'wav', 'mp4', 'mov', 'ogg', 'flac', 'mkv'];

class AppState {
	transcripts = $state<TranscriptMeta[]>([]);
	selected = $state<Transcript | null>(null);
	settings = $state<Settings | null>(null);
	phase = $state<AppPhase>({ kind: 'idle' });
	search = $state('');
	/** Most recent surfaced error; toast-style, dismissed by the next action. */
	lastError = $state<AppError | null>(null);

	/** Playback — meaningful only while `selected` has stored audio. */
	playerLoaded = $state(false);
	position = $state(0);
	playing = $state(false);
	peaks = $state<number[]>([]);

	/** Live recording: recent RMS levels for the waveform (newest last). */
	levelHistory = $state<number[]>([]);
	/** Segments streamed by the in-flight transcription job. */
	liveSegments = $state<EngineSegment[]>([]);
	/** The model download sheet, when open. */
	modelSheet = $state<ModelSheetState | null>(null);

	get filteredTranscripts(): TranscriptMeta[] {
		const q = this.search.trim().toLowerCase();
		if (!q) return this.transcripts;
		return this.transcripts.filter((t) => t.title.toLowerCase().includes(q));
	}

	get fxEnabled(): boolean {
		return this.settings?.fxEnabled ?? true;
	}

	async load(): Promise<void> {
		try {
			[this.transcripts, this.settings] = await Promise.all([
				api.listTranscripts(),
				api.getSettings()
			]);
		} catch {
			// Not running inside Tauri (or the store is unreadable) — keep the
			// shell alive with empty data.
			this.transcripts = [];
		}
	}

	async refreshTranscripts(): Promise<void> {
		try {
			this.transcripts = await api.listTranscripts();
		} catch (e) {
			this.fail(e);
		}
	}

	async select(id: string): Promise<void> {
		try {
			this.selected = await api.getTranscript(id);
		} catch (e) {
			this.fail(e);
			return;
		}
		// Recordings come with their archive WAV loaded and the scrubber
		// peaks ready; imports have no stored audio in v1.
		this.playerLoaded = false;
		this.position = 0;
		this.playing = false;
		this.peaks = [];
		if (this.selected?.audioRelativePath) {
			try {
				await api.playerLoad(id);
				this.playerLoaded = true;
				this.peaks = await api.playerPeaks(id);
			} catch (e) {
				this.fail(e);
			}
		}
	}

	async togglePlayback(): Promise<void> {
		if (!this.playerLoaded) return;
		try {
			if (this.playing) await api.playerPause();
			else await api.playerPlay();
		} catch (e) {
			this.fail(e);
		}
	}

	async seek(secs: number): Promise<void> {
		if (!this.playerLoaded) return;
		try {
			await api.playerSeek(secs);
		} catch (e) {
			this.fail(e);
		}
	}

	/** Persist one segment edit; the Rust side enforces the originalText rule. */
	async editSegment(segmentId: number, text: string): Promise<void> {
		if (!this.selected) return;
		try {
			this.selected = await api.updateTranscript(this.selected.id, segmentId, text);
			await this.refreshTranscripts();
		} catch (e) {
			this.fail(e);
		}
	}

	async rename(title: string): Promise<void> {
		if (!this.selected || !title.trim() || title === this.selected.title) return;
		try {
			this.selected = await api.renameTranscript(this.selected.id, title.trim());
			await this.refreshTranscripts();
		} catch (e) {
			this.fail(e);
		}
	}

	async toggleFx(): Promise<void> {
		if (!this.settings) return;
		this.settings.fxEnabled = !this.settings.fxEnabled;
		try {
			await api.setSettings($state.snapshot(this.settings) as Settings);
		} catch (e) {
			this.fail(e);
		}
	}

	// ------------------------------------------------ record → transcribe

	/** The ● REC button: start when idle, stop-and-transcribe when live. */
	async toggleRecord(): Promise<void> {
		if (this.phase.kind === 'recording') {
			await this.stopAndTranscribe();
		} else if (this.phase.kind === 'idle') {
			await this.startRecord();
		}
	}

	async startRecord(): Promise<void> {
		this.lastError = null;
		try {
			await api.startRecording(this.settings?.inputDeviceId ?? undefined);
			this.levelHistory = [];
			this.phase = { kind: 'recording', elapsedSecs: 0 };
		} catch (e) {
			this.fail(e);
		}
	}

	async stopAndTranscribe(): Promise<void> {
		try {
			await api.stopRecording();
		} catch (e) {
			this.fail(e);
			this.phase = { kind: 'idle' };
			return;
		}
		await this.transcribeSource({ type: 'recording' });
	}

	/** The + IMPORT button: pick a media file and transcribe it. */
	async importFile(): Promise<void> {
		if (this.phase.kind !== 'idle') return;
		const picked = await openFileDialog({
			multiple: false,
			filters: [{ name: 'Audio / video', extensions: IMPORT_EXTENSIONS }]
		}).catch(() => null);
		if (typeof picked !== 'string') return;
		await this.transcribeSource({ type: 'file', path: picked });
	}

	/** Shared tail of record/import: ensure the model, then start the job. */
	async transcribeSource(source: TranscribeSource): Promise<void> {
		this.lastError = null;
		const modelId = this.settings?.defaultModelId ?? 'tiny';
		let models: ModelInfo[] = [];
		try {
			models = await api.listModels();
		} catch (e) {
			this.fail(e);
			this.phase = { kind: 'idle' };
			return;
		}
		const model = models.find((m) => m.id === modelId) ?? models[0];
		if (model.state.state !== 'ready') {
			// The download sheet takes over; the job starts when it verifies.
			this.modelSheet = {
				modelId: model.id,
				sizeBytes: model.sizeBytes,
				bytes: model.state.state === 'partial' ? model.state.bytes : 0,
				total: model.sizeBytes,
				bytesPerSec: 0,
				pendingSource: source,
				verified: false,
				error: null
			};
			this.phase = { kind: 'idle' };
			api.downloadModel(model.id).catch(() => {
				// the model:download:error event carries the details
			});
			return;
		}
		await this.startJob(source, model.id);
	}

	private async startJob(source: TranscribeSource, modelId: string): Promise<void> {
		this.liveSegments = [];
		this.selected = null;
		const language = this.settings?.language;
		try {
			const jobId = await api.transcribe(
				source,
				modelId,
				language && language !== 'auto' ? language : undefined
			);
			this.phase = { kind: 'transcribing', jobId, pct: 0, phase: 'decode', modelId };
		} catch (e) {
			this.fail(e);
			this.phase = { kind: 'idle' };
		}
	}

	async cancelTranscribe(): Promise<void> {
		if (this.phase.kind !== 'transcribing') return;
		try {
			await api.cancelJob(this.phase.jobId);
		} catch (e) {
			this.fail(e);
		}
	}

	dismissModelSheet(): void {
		const sheet = this.modelSheet;
		this.modelSheet = null;
		if (sheet && !sheet.verified && !sheet.error) {
			api.cancelDownload(sheet.modelId).catch(() => {});
		}
	}

	/**
	 * Subscribe to every backend event stream. Called once at mount; the
	 * resolved function tears all of them down.
	 */
	wireEvents(): Promise<() => void> {
		const subs = [
			api.onPlaybackPosition((p) => {
				this.position = p.secs;
				this.playing = p.playing;
			}),
			api.onRecordLevel((l) => {
				if (this.phase.kind !== 'recording') return;
				this.phase = { kind: 'recording', elapsedSecs: l.elapsedSecs };
				this.levelHistory = [...this.levelHistory.slice(-239), l.rms];
			}),
			api.onTranscribeSegment((e) => {
				if (this.phase.kind === 'transcribing' && e.jobId === this.phase.jobId) {
					this.liveSegments = [...this.liveSegments, ...e.segments];
				}
			}),
			api.onTranscribeProgress((e) => {
				if (this.phase.kind === 'transcribing' && e.jobId === this.phase.jobId) {
					this.phase = { ...this.phase, pct: e.pct, phase: e.phase };
				}
			}),
			api.onTranscribeDone(async (e) => {
				if (this.phase.kind === 'transcribing' && e.jobId === this.phase.jobId) {
					this.phase = { kind: 'idle' };
					this.liveSegments = [];
					await this.refreshTranscripts();
					await this.select(e.transcriptId);
				}
			}),
			api.onTranscribeCancelled((jobId) => {
				if (this.phase.kind === 'transcribing' && jobId === this.phase.jobId) {
					this.phase = { kind: 'idle' };
					this.liveSegments = [];
				}
			}),
			api.onTranscribeError((e) => {
				if (this.phase.kind === 'transcribing' && e.jobId === this.phase.jobId) {
					this.phase = { kind: 'idle' };
					this.liveSegments = [];
					this.lastError = e.error;
				}
			}),
			api.onDownloadProgress((e) => {
				const sheet = this.modelSheet;
				if (!sheet || e.modelId !== sheet.modelId) return;
				// Smoothed transfer rate from successive events.
				const now = performance.now();
				const prev = this.lastProgressAt;
				if (prev && now > prev.at && e.bytes > prev.bytes) {
					const inst = ((e.bytes - prev.bytes) / (now - prev.at)) * 1000;
					sheet.bytesPerSec = sheet.bytesPerSec * 0.7 + inst * 0.3;
				}
				this.lastProgressAt = { at: now, bytes: e.bytes };
				sheet.bytes = e.bytes;
				sheet.total = e.total;
			}),
			api.onDownloadDone((modelId) => {
				const sheet = this.modelSheet;
				if (!sheet || modelId !== sheet.modelId) return;
				sheet.verified = true;
				const source = sheet.pendingSource;
				// Let the VERIFIED ✓ line land before the sheet yields.
				setTimeout(() => {
					this.modelSheet = null;
					this.startJob(source, modelId);
				}, 900);
			}),
			api.onDownloadError(([modelId, error]) => {
				const sheet = this.modelSheet;
				if (sheet && modelId === sheet.modelId) sheet.error = error;
			})
		];
		return Promise.all(subs).then((fns) => () => fns.forEach((fn) => fn()));
	}

	private lastProgressAt: { at: number; bytes: number } | null = null;

	fail(e: unknown): void {
		this.lastError = api.isAppError(e) ? e : { kind: 'store', message: String(e) };
	}
}

export const app = new AppState();
