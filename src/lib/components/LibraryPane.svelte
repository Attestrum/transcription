<script lang="ts">
	import { app } from '../app-state.svelte';

	function meta(durationSecs: number, updatedAt: string, modelId: string): string {
		const mins = Math.floor(durationSecs / 60);
		const secs = Math.floor(durationSecs % 60);
		const when = updatedAt.slice(0, 10);
		return `${mins}:${String(secs).padStart(2, '0')} · ${when} · ${modelId}`;
	}
</script>

<aside class="library">
	<div class="search">
		<span class="prompt">&gt;</span>
		<input
			type="text"
			bind:value={app.search}
			placeholder="_"
			spellcheck="false"
			aria-label="Search transcripts"
		/>
	</div>

	{#if app.filteredTranscripts.length === 0}
		<div class="empty">
			{#if app.transcripts.length === 0}
				<pre aria-hidden="true">{`   ┌─────────────┐
   │  ▁▂▅▂▇▂▅▂▁  │
   └─────────────┘`}</pre>
				<p>NO TRANSCRIPTS YET</p>
				<p class="hint">hit ● REC or + IMPORT</p>
			{:else}
				<p>NO MATCHES</p>
				<p class="hint">for "{app.search}"</p>
			{/if}
		</div>
	{:else}
		<ul role="listbox" aria-label="Transcripts">
			{#each app.filteredTranscripts as t (t.id)}
				<li>
					<button
						class="row"
						class:selected={app.selected?.id === t.id}
						onclick={() => app.select(t.id)}
						role="option"
						aria-selected={app.selected?.id === t.id}
					>
						<span class="title">{t.title}</span>
						<span class="meta">{meta(t.duration, t.updatedAt, t.modelId)}</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</aside>

<style>
	.library {
		width: 280px;
		flex: none;
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--hairline);
		background: var(--panel);
		overflow: hidden;
	}

	.search {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 14px;
		border-bottom: 1px solid var(--hairline);
	}

	.prompt {
		color: var(--green);
		font-size: 13px;
	}

	input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		color: var(--text);
		font-family: var(--mono);
		font-size: 13px;
		caret-color: var(--cyan);
	}

	input::placeholder {
		color: var(--text-dim);
	}

	ul {
		flex: 1;
		margin: 0;
		padding: 0;
		list-style: none;
		overflow-y: auto;
	}

	.row {
		display: flex;
		flex-direction: column;
		gap: 3px;
		width: 100%;
		padding: 10px 14px;
		background: transparent;
		border: none;
		border-left: 2px solid transparent;
		color: var(--text);
		font-family: var(--mono);
		text-align: left;
		cursor: pointer;
		transition:
			border-color var(--t-fast) var(--ease),
			background var(--t-fast) var(--ease);
	}

	.row:hover {
		border-left-color: var(--cyan-dim);
		background: var(--cyan-wash);
	}

	.row.selected {
		border-left-color: var(--cyan);
		background: var(--cyan-wash);
	}

	.title {
		font-size: 13px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.meta {
		font-size: 10px;
		letter-spacing: 0.08em;
		color: var(--green-dim);
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		color: var(--text-dim);
	}

	.empty pre {
		margin: 0 0 8px;
		font-size: 11px;
		line-height: 1.3;
		color: var(--cyan-dim);
	}

	.empty p {
		margin: 0;
		font-size: 11px;
		letter-spacing: 0.2em;
	}

	.empty .hint {
		font-size: 10px;
		letter-spacing: 0.1em;
		color: var(--text-dim);
	}
</style>
