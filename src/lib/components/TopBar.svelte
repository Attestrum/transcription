<script lang="ts">
	import { app } from '../app-state.svelte';

	function statusText(): string {
		const p = app.phase;
		switch (p.kind) {
			case 'idle':
				return 'IDLE';
			case 'recording':
				return `REC ${hms(p.elapsedSecs)}`;
			case 'transcribing':
				return `TRANSCRIBING ${p.pct}% · ${p.modelId}${p.phase === 'decode' ? ' · decoding' : ''}`;
		}
	}

	function hms(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}`;
	}
</script>

<header class="topbar">
	<div class="wordmark">
		<span class="brand">ATTESTRUM</span>
		<span class="sep">▸</span>
		<span class="product">TRANSCRIPTION</span>
	</div>

	<div class="status" class:recording={app.phase.kind === 'recording'}>
		{statusText()}<span class="cursor">▮</span>
	</div>

	<div class="actions">
		<button
			class="rec"
			class:live={app.phase.kind === 'recording'}
			disabled={app.phase.kind === 'transcribing'}
			title={app.phase.kind === 'recording' ? 'Stop recording' : 'Record'}
			onclick={() => app.toggleRecord()}
		>
			● REC
		</button>
		<button
			class="import"
			disabled={app.phase.kind !== 'idle'}
			title="Import audio or video"
			onclick={() => app.importFile()}
		>
			+ IMPORT
		</button>
	</div>
</header>

<style>
	.topbar {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		height: 48px;
		padding: 0 16px;
		border-bottom: 1px solid var(--hairline);
		background: var(--panel);
	}

	.wordmark {
		font-size: 13px;
		letter-spacing: 0.28em;
		white-space: nowrap;
	}

	.brand {
		color: var(--cyan);
		text-shadow: var(--glow-cyan);
	}

	.sep {
		color: var(--text-dim);
		margin: 0 5px;
	}

	.product {
		color: var(--green);
		text-shadow: var(--glow-green);
	}

	.status {
		justify-self: center;
		font-size: 12px;
		letter-spacing: 0.2em;
		color: var(--text-dim);
		white-space: nowrap;
	}

	.status.recording {
		color: var(--green);
		text-shadow: var(--glow-green);
	}

	.cursor {
		color: var(--green);
		margin-left: 6px;
		animation: cursor-blink 1.2s step-end infinite;
	}

	.actions {
		justify-self: end;
		display: flex;
		gap: 10px;
	}

	button {
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.18em;
		padding: 6px 12px;
		background: transparent;
		border: 1px solid var(--hairline);
		color: var(--text);
		cursor: pointer;
		transition:
			border-color var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease),
			box-shadow var(--t-fast) var(--ease),
			transform var(--t-fast) var(--ease);
	}

	button:hover {
		border-color: var(--cyan-dim);
		color: var(--cyan);
		box-shadow: var(--glow-cyan);
	}

	button:active {
		transform: translateY(1px);
	}

	.rec {
		color: var(--green);
	}

	.rec:hover {
		border-color: var(--green-dim);
		color: var(--green);
		box-shadow: var(--glow-green);
	}

	.rec.live {
		border-color: var(--green-dim);
		box-shadow: var(--glow-green);
		animation: rec-pulse 1.6s var(--ease) infinite;
	}

	@keyframes rec-pulse {
		0%,
		100% {
			box-shadow: var(--glow-green);
		}
		50% {
			box-shadow: 0 0 12px rgba(127, 255, 176, 0.55);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.cursor,
		.rec.live {
			animation: none;
		}
	}
</style>
