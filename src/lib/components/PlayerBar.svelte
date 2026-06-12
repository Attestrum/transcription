<script lang="ts">
	import { app } from '../app-state.svelte';

	let canvas = $state<HTMLCanvasElement | null>(null);

	const duration = $derived(app.selected?.duration ?? 0);

	function fmt(secs: number): string {
		const m = Math.floor(secs / 60);
		const s = secs % 60;
		return `${String(m).padStart(2, '0')}:${s.toFixed(1).padStart(4, '0')}`;
	}

	function seekFromPointer(e: PointerEvent) {
		if (!canvas || duration <= 0) return;
		const rect = canvas.getBoundingClientRect();
		const frac = Math.min(Math.max((e.clientX - rect.left) / rect.width, 0), 1);
		app.seek(frac * duration);
	}

	// Redraw the peaks strip whenever peaks or position change.
	$effect(() => {
		const el = canvas;
		if (!el) return;
		const peaks = app.peaks;
		const playedFrac = duration > 0 ? app.position / duration : 0;
		const dpr = window.devicePixelRatio || 1;
		const w = el.clientWidth;
		const h = el.clientHeight;
		el.width = w * dpr;
		el.height = h * dpr;
		const ctx = el.getContext('2d');
		if (!ctx) return;
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, w, h);
		const n = peaks.length;
		if (n === 0) return;
		const mid = h / 2;
		const barW = w / n;
		const played = getComputedStyle(el).getPropertyValue('--cyan').trim() || '#7fe0ff';
		const rest = 'rgba(127, 224, 255, 0.25)';
		for (let i = 0; i < n; i++) {
			const amp = Math.max(peaks[i] * (h / 2 - 1), 1);
			ctx.fillStyle = i / n <= playedFrac ? played : rest;
			ctx.fillRect(i * barW, mid - amp, Math.max(barW - 0.5, 0.5), amp * 2);
		}
	});
</script>

<div class="player">
	<button class="toggle" onclick={() => app.togglePlayback()}>
		{app.playing ? '[ ⏸ ]' : '[ ▶ ]'}
	</button>
	<span class="time">{fmt(app.position)}</span>
	<canvas
		bind:this={canvas}
		class="strip"
		onpointerdown={seekFromPointer}
		aria-label="Seek within the recording"
	></canvas>
	<span class="time total">{fmt(duration)}</span>
</div>

<style>
	.player {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 52px;
		padding: 0 16px;
		border-top: 1px solid var(--hairline);
		background: var(--panel);
	}

	.toggle {
		padding: 4px 6px;
		background: none;
		border: none;
		font-family: var(--mono);
		font-size: 13px;
		color: var(--cyan);
		cursor: pointer;
		transition: text-shadow var(--t-fast) var(--ease);
	}

	.toggle:hover {
		text-shadow: var(--glow-cyan);
	}

	.time {
		font-size: 11px;
		letter-spacing: 0.1em;
		color: var(--green);
		min-width: 58px;
		text-align: center;
	}

	.time.total {
		color: var(--text-dim);
	}

	.strip {
		flex: 1;
		height: 36px;
		cursor: pointer;
	}
</style>
