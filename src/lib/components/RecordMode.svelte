<script lang="ts">
	import { app } from '../app-state.svelte';

	const elapsed = $derived(app.phase.kind === 'recording' ? app.phase.elapsedSecs : 0);

	/** "MM:SS" under an hour, "H:MM:SS" after — the original's timer. */
	function timer(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
	}
</script>

<!-- The recording sheet (RecordSourceSheet's recording screen, mic source). -->
<div class="scrim" role="dialog" aria-modal="true" aria-label="Recording">
	<div class="sheet">
		<svg class="mic" width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor"
			stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
			<circle cx="12" cy="12" r="11" />
			<rect x="10" y="5.5" width="4" height="7.5" rx="2" fill="currentColor" stroke="none" />
			<path d="M7.5 11a4.5 4.5 0 0 0 9 0" />
			<line x1="12" y1="15.5" x2="12" y2="18" />
		</svg>
		<h2>Record from microphone</h2>
		<p class="desc">Captures your selected input device. Recording stays on this Mac.</p>
		<div class="timer">{timer(elapsed)}</div>
		<div class="buttons">
			<button class="stop" onclick={() => app.toggleRecord()}>
				<svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
					<rect x="5" y="5" width="14" height="14" rx="2" />
				</svg>
				Stop &amp; Transcribe
			</button>
			<button class="cancel" onclick={() => app.cancelRecording()}>Cancel</button>
		</div>
	</div>
</div>

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.45);
	}

	.sheet {
		width: 440px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		padding: 28px;
		background: var(--surface-base);
		border: 1px solid var(--border-strong);
		border-radius: 12px;
		box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
	}

	.mic {
		color: var(--red);
		animation: pulse 1.6s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.55;
		}
	}

	h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
	}

	.desc {
		margin: 0;
		font-size: 12px;
		color: var(--label-secondary);
		text-align: center;
	}

	.timer {
		font-family: var(--mono);
		font-size: 32px;
		color: var(--red);
		font-variant-numeric: tabular-nums;
	}

	.buttons {
		display: flex;
		gap: 10px;
	}

	.stop {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 8px 16px;
		background: var(--accent);
		border: none;
		border-radius: 7px;
		font-size: 13px;
		font-weight: 500;
		color: #fff;
		cursor: pointer;
	}

	.stop:hover {
		filter: brightness(1.1);
	}

	.cancel {
		padding: 8px 16px;
		background: var(--surface-elevated);
		border: 1px solid var(--border-strong);
		border-radius: 7px;
		font-size: 13px;
		color: var(--label-primary);
		cursor: pointer;
	}

	.cancel:hover {
		background: var(--surface-highlight);
	}

	@media (prefers-reduced-motion: reduce) {
		.mic {
			animation: none;
		}
	}
</style>
