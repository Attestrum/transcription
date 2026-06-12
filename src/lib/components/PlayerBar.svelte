<script lang="ts">
	import { app } from '../app-state.svelte';

	const duration = $derived(app.selected?.duration ?? 0);

	/** "M:SS" / "H:MM:SS", tabular digits — AudioPlayerBar's format. */
	function fmt(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
	}
</script>

<div class="player">
	<button
		class="toggle"
		title={app.playing ? 'Pause (Space)' : 'Play (Space)'}
		aria-label={app.playing ? 'Pause' : 'Play'}
		onclick={() => app.togglePlayback()}
	>
		{#if app.playing}
			<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
				<rect x="5" y="4" width="5" height="16" rx="1" /><rect x="14" y="4" width="5" height="16" rx="1" />
			</svg>
		{:else}
			<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
				<path d="M7 4.5v15a1 1 0 0 0 1.5.87l13-7.5a1 1 0 0 0 0-1.74l-13-7.5A1 1 0 0 0 7 4.5z" />
			</svg>
		{/if}
	</button>
	<span class="time current">{fmt(app.position)}</span>
	<input
		class="scrubber"
		type="range"
		min="0"
		max={Math.max(duration, 0.01)}
		step="0.1"
		value={app.position}
		oninput={(e) => app.seek(Number((e.target as HTMLInputElement).value))}
		aria-label="Seek"
	/>
	<span class="time total">{fmt(duration)}</span>
</div>

<style>
	.player {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 50px;
		padding: 0 16px;
		border-top: 1px solid var(--border);
		flex: none;
	}

	.toggle {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		border-radius: 7px;
		background: var(--accent);
		color: #fff;
		cursor: pointer;
	}

	.toggle:hover {
		filter: brightness(1.1);
	}

	.toggle:active {
		filter: brightness(0.9);
	}

	.time {
		width: 48px;
		font-size: 11px;
		color: var(--label-secondary);
		font-variant-numeric: tabular-nums;
	}

	.time.current {
		text-align: right;
	}

	.scrubber {
		flex: 1;
		appearance: none;
		height: 4px;
		border-radius: 2px;
		background: var(--surface-highlight);
		outline: none;
		cursor: pointer;
	}

	.scrubber::-webkit-slider-thumb {
		appearance: none;
		width: 13px;
		height: 13px;
		border-radius: 50%;
		background: #fff;
		border: none;
		box-shadow: 0 0 2px rgba(0, 0, 0, 0.5);
	}
</style>
