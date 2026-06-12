<script lang="ts">
	import { app } from '../app-state.svelte';

	const sheet = $derived(app.modelSheet);

	function mb(n: number): string {
		return (n / (1024 * 1024)).toFixed(1);
	}

	function pct(bytes: number, total: number): number {
		return total > 0 ? Math.min((bytes / total) * 100, 100) : 0;
	}
</script>

{#if sheet}
	<div class="scrim" role="dialog" aria-modal="true" aria-label="Model download">
		<div class="sheet">
			<h2>Downloading {sheet.modelId} model</h2>
			<p class="desc">
				One-time download from Hugging Face — {mb(sheet.sizeBytes)} MB. Verified before use;
				transcription starts automatically.
			</p>
			<div class="bar">
				<div class="fill" style:width="{pct(sheet.bytes, sheet.total)}%"></div>
			</div>
			<p class="progress">
				{mb(sheet.bytes)} of {mb(sheet.total)} MB
				{#if sheet.bytesPerSec > 0}· {mb(sheet.bytesPerSec)} MB/s{/if}
			</p>
			{#if sheet.error}
				<p class="error">{sheet.error.message}</p>
				<p class="desc">The download resumes from where it stopped — try again.</p>
			{:else if sheet.verified}
				<p class="verified">
					<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
						stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M20 6 9 17l-5-5" />
					</svg>
					Checksum verified
				</p>
			{/if}
			<button class="cancel" onclick={() => app.dismissModelSheet()}>
				{sheet.error ? 'Close' : 'Cancel'}
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
		background: rgba(0, 0, 0, 0.45);
	}

	.sheet {
		width: 440px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 28px;
		background: var(--surface-base);
		border: 1px solid var(--border-strong);
		border-radius: 12px;
		box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
	}

	h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
	}

	.desc {
		margin: 0;
		font-size: 12px;
		line-height: 1.45;
		color: var(--label-secondary);
	}

	.bar {
		height: 6px;
		border-radius: 3px;
		background: var(--surface-highlight);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		border-radius: 3px;
		background: var(--accent);
		transition: width 0.25s linear;
	}

	.progress {
		margin: 0;
		font-size: 11px;
		color: var(--label-secondary);
		font-variant-numeric: tabular-nums;
	}

	.verified {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0;
		font-size: 12px;
		font-weight: 500;
		color: var(--green);
	}

	.error {
		margin: 0;
		font-size: 12px;
		color: var(--red);
	}

	.cancel {
		align-self: flex-end;
		padding: 7px 14px;
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
</style>
