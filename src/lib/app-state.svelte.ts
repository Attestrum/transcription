/**
 * Central UI state (Svelte 5 runes). One instance app-wide; components read
 * fields directly and call the mutation helpers.
 *
 * Outside the Tauri webview (plain-vite dev in a browser) every invoke()
 * rejects — load() degrades to empty data so the shell stays inspectable.
 */

import * as api from './api';
import type { AppError, Settings, Transcript, TranscriptMeta } from './api';

export type AppPhase =
	| { kind: 'idle' }
	| { kind: 'recording'; elapsedSecs: number }
	| { kind: 'transcribing'; jobId: number; pct: number; phase: 'decode' | 'transcribe'; modelId: string };

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

	fail(e: unknown): void {
		this.lastError = api.isAppError(e) ? e : { kind: 'store', message: String(e) };
	}
}

export const app = new AppState();
