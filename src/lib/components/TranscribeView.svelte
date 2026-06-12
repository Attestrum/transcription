<script lang="ts">
	import { app } from '../app-state.svelte';

	const phase = $derived(app.phase.kind === 'transcribing' ? app.phase : null);

	let revealed = $state(0); // segments fully typed in
	let typing = $state(''); // visible prefix of the segment being typed
	let scroller = $state<HTMLDivElement | null>(null);

	// Typewriter: each freshly streamed segment types in over ~120 ms.
	// With FX off (or reduced motion) everything shows instantly.
	$effect(() => {
		const total = app.liveSegments.length;
		if (!app.fxEnabled || matchMedia('(prefers-reduced-motion: reduce)').matches) {
			revealed = total;
			typing = '';
			return;
		}
		if (revealed >= total || typing !== '') return;
		const text = app.liveSegments[revealed].text;
		const started = performance.now();
		let raf = 0;
		const step = (now: number) => {
			const frac = Math.min((now - started) / 120, 1);
			typing = text.slice(0, Math.ceil(text.length * frac));
			if (frac < 1) {
				raf = requestAnimationFrame(step);
			} else {
				revealed += 1;
				typing = '';
			}
		};
		raf = requestAnimationFrame(step);
		return () => cancelAnimationFrame(raf);
	});

	// Keep the newest line in view as segments stream.
	$effect(() => {
		void revealed;
		void typing;
		scroller?.scrollTo({ top: scroller.scrollHeight });
	});

	function ts(secs: number): string {
		const t = Math.floor(secs * 1000);
		const h = Math.floor(t / 3_600_000);
		const m = Math.floor(t / 60_000) % 60;
		const s = Math.floor(t / 1000) % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}`;
	}
</script>

<section class="transcribing">
	<header class="head">
		<span class="state">
			{phase?.phase === 'decode' ? 'DECODING' : 'TRANSCRIBING'}
			{phase ? `${phase.pct}%` : ''} · {phase?.modelId}
		</span>
		<button class="cancel" onclick={() => app.cancelTranscribe()}>[ ✕ CANCEL ]</button>
	</header>

	<div class="stream" bind:this={scroller}>
		{#each app.liveSegments.slice(0, revealed) as seg, i (i)}
			<div class="row">
				<span class="stamp">[{ts(seg.start)}]</span>
				<span class="text">{seg.text}</span>
			</div>
		{/each}
		{#if typing}
			<div class="row">
				<span class="stamp">[{ts(app.liveSegments[revealed].start)}]</span>
				<span class="text">{typing}<span class="cursor">▮</span></span>
			</div>
		{/if}
		<div class="row ghost">
			<span class="text">▮ decoding…</span>
		</div>
	</div>
</section>

<style>
	.transcribing {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg);
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 24px;
		border-bottom: 1px solid var(--hairline);
	}

	.state {
		font-size: 12px;
		letter-spacing: 0.22em;
		color: var(--cyan);
		text-shadow: var(--glow-cyan);
	}

	.cancel {
		padding: 4px 10px;
		background: transparent;
		border: 1px solid var(--hairline);
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.14em;
		color: var(--text-dim);
		cursor: pointer;
		transition: color var(--t-fast) var(--ease);
	}

	.cancel:hover {
		color: var(--red);
		border-color: var(--red);
	}

	.stream {
		flex: 1;
		overflow-y: auto;
		padding: 16px 24px;
	}

	.row {
		display: flex;
		gap: 12px;
		padding: 5px 0;
		line-height: 1.55;
		font-size: 13px;
	}

	.stamp {
		flex: none;
		color: var(--green);
		font-size: 12px;
	}

	.text {
		color: var(--text);
	}

	.cursor {
		color: var(--green);
		margin-left: 2px;
	}

	.ghost .text {
		color: var(--text-dim);
		opacity: 0.6;
	}
</style>
