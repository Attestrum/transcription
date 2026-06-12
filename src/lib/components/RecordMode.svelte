<script lang="ts">
	import { app } from '../app-state.svelte';

	let canvas = $state<HTMLCanvasElement | null>(null);

	const elapsed = $derived(app.phase.kind === 'recording' ? app.phase.elapsedSecs : 0);

	function timer(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}`;
	}

	// Mirrored-bar waveform: newest level on the right, additive glow,
	// decaying peak caps.
	let caps: number[] = [];
	$effect(() => {
		const el = canvas;
		if (!el) return;
		const levels = app.levelHistory;
		const dpr = window.devicePixelRatio || 1;
		const w = el.clientWidth;
		const h = el.clientHeight;
		el.width = w * dpr;
		el.height = h * dpr;
		const ctx = el.getContext('2d');
		if (!ctx) return;
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, w, h);

		const n = 120;
		const recent = levels.slice(-n);
		const barW = w / n;
		const mid = h / 2;
		if (caps.length !== n) caps = new Array(n).fill(0);

		ctx.globalCompositeOperation = 'lighter';
		for (let i = 0; i < recent.length; i++) {
			// Map RMS (speech peaks ~0.3) onto the bar height with headroom.
			const amp = Math.min(recent[i] * 3.2, 1) * (mid - 6) + 2;
			const x = w - (recent.length - i) * barW;
			ctx.fillStyle = 'rgba(127, 255, 176, 0.55)';
			ctx.fillRect(x, mid - amp, Math.max(barW - 1.5, 1), amp * 2);

			// Decaying peak caps.
			const slot = n - (recent.length - i);
			caps[slot] = Math.max(caps[slot] * 0.96, amp);
			ctx.fillStyle = 'rgba(127, 255, 176, 0.9)';
			ctx.fillRect(x, mid - caps[slot] - 2, Math.max(barW - 1.5, 1), 1.5);
			ctx.fillRect(x, mid + caps[slot] + 0.5, Math.max(barW - 1.5, 1), 1.5);
		}
		ctx.globalCompositeOperation = 'source-over';
	});
</script>

<section class="record">
	<div class="live">● RECORDING</div>
	<canvas bind:this={canvas} class="wave" aria-hidden="true"></canvas>
	<div class="timer">{timer(elapsed)}</div>
	<button class="stop" onclick={() => app.toggleRecord()}>[ ■ STOP &amp; TRANSCRIBE ]</button>
</section>

<style>
	.record {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 28px;
		background: var(--bg);
	}

	.live {
		font-size: 12px;
		letter-spacing: 0.32em;
		color: var(--green);
		text-shadow: var(--glow-green);
		animation: rec-blink 1.6s step-end infinite;
	}

	@keyframes rec-blink {
		0%,
		74% {
			opacity: 1;
		}
		75%,
		100% {
			opacity: 0.35;
		}
	}

	.wave {
		width: min(86%, 900px);
		height: 160px;
	}

	.timer {
		font-size: 56px;
		font-weight: 600;
		letter-spacing: 0.12em;
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}

	.stop {
		padding: 10px 18px;
		background: transparent;
		border: 1px solid var(--green-dim);
		font-family: var(--mono);
		font-size: 12px;
		letter-spacing: 0.18em;
		color: var(--green);
		cursor: pointer;
		transition:
			box-shadow var(--t-fast) var(--ease),
			transform var(--t-fast) var(--ease);
	}

	.stop:hover {
		box-shadow: var(--glow-green);
	}

	.stop:active {
		transform: translateY(1px);
	}

	@media (prefers-reduced-motion: reduce) {
		.live {
			animation: none;
		}
	}
</style>
