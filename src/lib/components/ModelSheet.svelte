<script lang="ts">
	import { app } from '../app-state.svelte';

	const sheet = $derived(app.modelSheet);

	function bar(bytes: number, total: number): string {
		const width = 28;
		const filled = total > 0 ? Math.round((bytes / total) * width) : 0;
		return '█'.repeat(filled) + '░'.repeat(width - filled);
	}

	function pct(bytes: number, total: number): number {
		return total > 0 ? Math.floor((bytes / total) * 100) : 0;
	}

	function mb(n: number): string {
		return (n / (1024 * 1024)).toFixed(1);
	}
</script>

{#if sheet}
	<div class="scrim" role="dialog" aria-modal="true" aria-label="Model download">
		<div class="sheet">
			<p class="line dim">&gt; model {sheet.modelId} required — fetching from huggingface.co</p>
			<p class="line dim">&gt; ggml-{sheet.modelId}.bin · {mb(sheet.sizeBytes)} MB</p>
			<p class="line">
				{bar(sheet.bytes, sheet.total)}
				{pct(sheet.bytes, sheet.total)}%
				{#if sheet.bytesPerSec > 0}· {mb(sheet.bytesPerSec)} MB/s{/if}
			</p>
			{#if sheet.error}
				<p class="line error">! {sheet.error.message}</p>
				<p class="line dim">download resumes from where it stopped — try again</p>
			{:else if sheet.verified}
				<p class="line verified">SHA-256 … VERIFIED ✓</p>
				<p class="line dim">starting transcription<span class="cursor">▮</span></p>
			{:else}
				<p class="line dim">SHA-256 streaming…<span class="cursor">▮</span></p>
			{/if}
			<button class="dismiss" onclick={() => app.dismissModelSheet()}>
				{sheet.error ? '[ CLOSE ]' : '[ CANCEL ]'}
			</button>
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(10, 14, 20, 0.82);
	}

	.sheet {
		width: min(560px, 90vw);
		padding: 24px 28px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		box-shadow: var(--glow-cyan-strong);
	}

	.line {
		margin: 0 0 10px;
		font-size: 12px;
		letter-spacing: 0.06em;
		color: var(--cyan);
		white-space: nowrap;
		overflow: hidden;
	}

	.line.dim {
		color: var(--text-dim);
	}

	.line.verified {
		color: var(--green);
		text-shadow: var(--glow-green);
	}

	.line.error {
		color: var(--red);
		white-space: normal;
	}

	.cursor {
		color: var(--green);
		margin-left: 5px;
		animation: cursor-blink 1.2s step-end infinite;
	}

	.dismiss {
		margin-top: 8px;
		padding: 6px 12px;
		background: transparent;
		border: 1px solid var(--hairline);
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.16em;
		color: var(--text-dim);
		cursor: pointer;
		transition: color var(--t-fast) var(--ease);
	}

	.dismiss:hover {
		color: var(--cyan);
		border-color: var(--cyan-dim);
	}

	@media (prefers-reduced-motion: reduce) {
		.cursor {
			animation: none;
		}
	}
</style>
