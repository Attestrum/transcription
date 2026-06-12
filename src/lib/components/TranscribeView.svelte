<script lang="ts">
	import { app } from '../app-state.svelte';

	const phase = $derived(app.phase.kind === 'transcribing' ? app.phase : null);

	let scroller = $state<HTMLDivElement | null>(null);

	$effect(() => {
		void app.liveSegments.length;
		scroller?.scrollTo({ top: scroller.scrollHeight });
	});

	function ts(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
	}
</script>

<div class="transcribing">
	<header class="head">
		<div class="status">
			<svg class="wave" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
				stroke-width="2" stroke-linecap="round" aria-hidden="true">
				<line x1="4" y1="10" x2="4" y2="14" /><line x1="9" y1="6" x2="9" y2="18" />
				<line x1="14" y1="3" x2="14" y2="21" /><line x1="19" y1="8" x2="19" y2="16" />
			</svg>
			<span>
				{phase?.phase === 'decode' ? 'Decoding' : 'Transcribing'}…
				{phase && phase.pct > 0 ? `${phase.pct}%` : ''}
			</span>
		</div>
		<button class="cancel" onclick={() => app.cancelTranscribe()}>Cancel</button>
	</header>
	<div class="bar">
		<div class="fill" style:width="{phase?.pct ?? 0}%"></div>
	</div>

	<div class="stream" bind:this={scroller}>
		{#each app.liveSegments as seg, i (i)}
			<div class="row">
				<span class="stamp">{ts(seg.start)}</span>
				<span class="text">{seg.text}</span>
			</div>
		{/each}
	</div>
</div>

<style>
	.transcribing {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-height: 0;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 20px 10px;
	}

	.status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		font-weight: 500;
		color: var(--label-primary);
		font-variant-numeric: tabular-nums;
	}

	.wave {
		color: var(--accent);
		animation: pulse 1.4s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.45;
		}
	}

	.cancel {
		padding: 5px 12px;
		background: var(--surface-elevated);
		border: 1px solid var(--border-strong);
		border-radius: 6px;
		font-size: 12px;
		color: var(--label-primary);
		cursor: pointer;
	}

	.cancel:hover {
		background: var(--surface-highlight);
	}

	.bar {
		height: 6px;
		margin: 0 20px 8px;
		border-radius: 3px;
		background: var(--surface-highlight);
		overflow: hidden;
		flex: none;
	}

	.fill {
		height: 100%;
		border-radius: 3px;
		background: var(--accent);
		transition: width 0.25s linear;
	}

	.stream {
		flex: 1;
		overflow-y: auto;
		padding: 12px 16px;
	}

	.row {
		display: flex;
		gap: 10px;
		padding: 4px 6px;
		line-height: 1.5;
	}

	.stamp {
		flex: none;
		width: 64px;
		font-family: var(--mono);
		font-size: 11px;
		text-align: right;
		color: var(--label-tertiary);
		line-height: inherit;
	}

	.text {
		font-size: 14px;
		color: var(--label-primary);
	}

	@media (prefers-reduced-motion: reduce) {
		.wave {
			animation: none;
		}
	}
</style>
