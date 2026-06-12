<script lang="ts">
	import { app } from '../app-state.svelte';

	function ts(secs: number): string {
		const t = Math.floor(secs * 1000);
		const h = Math.floor(t / 3_600_000);
		const m = Math.floor(t / 60_000) % 60;
		const s = Math.floor(t / 1000) % 60;
		const ms = t % 1000;
		const pad = (n: number, w = 2) => String(n).padStart(w, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}.${pad(ms, 3)}`;
	}
</script>

<section class="editor">
	{#if app.selected}
		{@const t = app.selected}
		<header class="doc-head">
			<h1>{t.title}</h1>
			<div class="chips">
				<span class="chip">{t.modelId}</span>
				<span class="chip">{t.language.toUpperCase()}</span>
				<span class="chip">{Math.floor(t.duration / 60)}:{String(Math.floor(t.duration % 60)).padStart(2, '0')}</span>
				{#if t.audioRelativePath}<span class="chip green">WAV</span>{/if}
			</div>
		</header>
		<div class="segments">
			{#each t.segments as seg (seg.id)}
				<div class="segment">
					<button class="stamp" title="Seek (playback lands in M8)">[{ts(seg.start)}]</button>
					<span class="text">{seg.text}</span>
				</div>
			{/each}
		</div>
	{:else}
		<div class="empty">
			<pre aria-hidden="true">{`╔══════════════════════════╗
║                          ║
║   SELECT A TRANSCRIPT    ║
║   or start a recording   ║
║                          ║
╚══════════════════════════╝`}</pre>
		</div>
	{/if}
</section>

<style>
	.editor {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg);
	}

	.doc-head {
		padding: 18px 24px 14px;
		border-bottom: 1px solid var(--hairline);
	}

	h1 {
		margin: 0 0 8px;
		font-size: 16px;
		font-weight: 600;
		letter-spacing: 0.04em;
	}

	.chips {
		display: flex;
		gap: 8px;
	}

	.chip {
		font-size: 10px;
		letter-spacing: 0.14em;
		padding: 2px 8px;
		border: 1px solid var(--hairline);
		color: var(--text-dim);
	}

	.chip.green {
		color: var(--green);
		border-color: var(--green-dim);
	}

	.segments {
		flex: 1;
		overflow-y: auto;
		padding: 16px 24px;
	}

	.segment {
		display: flex;
		gap: 12px;
		padding: 6px 0;
		line-height: 1.55;
	}

	.stamp {
		flex: none;
		padding: 0;
		background: none;
		border: none;
		font-family: var(--mono);
		font-size: 12px;
		color: var(--green);
		cursor: pointer;
		transition: text-shadow var(--t-fast) var(--ease);
	}

	.stamp:hover {
		text-shadow: var(--glow-green);
	}

	.text {
		font-size: 13px;
		user-select: text;
	}

	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.empty pre {
		margin: 0;
		font-size: 12px;
		line-height: 1.5;
		color: var(--text-dim);
	}
</style>
