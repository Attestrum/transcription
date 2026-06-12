<script lang="ts">
	import { app } from '../app-state.svelte';
	import type { TranscriptMeta } from '../api';

	let hovered = $state<string | null>(null);

	/** "5m 20s" / "1h 7m 23s" — the original's abbreviated duration. */
	function dur(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		if (h > 0) return `${h}h ${m}m ${s}s`;
		if (m > 0) return `${m}m ${s}s`;
		return `${s}s`;
	}

	/** "6/6/26" */
	function shortDate(iso: string): string {
		const d = new Date(iso);
		return `${d.getMonth() + 1}/${d.getDate()}/${String(d.getFullYear()).slice(2)}`;
	}

	function compactModel(id: string): string {
		const names: Record<string, string> = {
			tiny: 'Tiny',
			base: 'Base',
			small: 'Small',
			'large-v3-turbo': 'Turbo'
		};
		return names[id] ?? id;
	}

	/** Today / Yesterday / Earlier, in that order, empty groups dropped. */
	const groups = $derived.by(() => {
		const today: TranscriptMeta[] = [];
		const yesterday: TranscriptMeta[] = [];
		const earlier: TranscriptMeta[] = [];
		const now = new Date();
		const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
		const startOfYesterday = startOfToday - 86_400_000;
		for (const t of app.filteredTranscripts) {
			const created = new Date(t.createdAt).getTime();
			if (created >= startOfToday) today.push(t);
			else if (created >= startOfYesterday) yesterday.push(t);
			else earlier.push(t);
		}
		return [
			{ label: 'Today', items: today },
			{ label: 'Yesterday', items: yesterday },
			{ label: 'Earlier', items: earlier }
		].filter((g) => g.items.length > 0);
	});
</script>

<aside class="sidebar">
	<!-- Top strip stays clear of the overlay traffic lights and drags the window. -->
	<div class="traffic-spacer" data-tauri-drag-region></div>

	<div class="search">
		<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
			stroke-width="2.2" stroke-linecap="round" aria-hidden="true">
			<circle cx="11" cy="11" r="7" />
			<line x1="21" y1="21" x2="16.5" y2="16.5" />
		</svg>
		<input
			type="text"
			bind:value={app.search}
			placeholder="Search transcripts"
			spellcheck="false"
			aria-label="Search transcripts"
		/>
	</div>

	<div class="list" role="listbox" aria-label="Transcripts">
		{#if app.transcripts.length === 0}
			<p class="empty">No transcripts yet.<br />Record or drop an audio file.</p>
		{:else if groups.length === 0}
			<p class="empty">No matches for “{app.search}”.</p>
		{/if}
		{#each groups as group (group.label)}
			<p class="group-label">{group.label}</p>
			{#each group.items as t (t.id)}
				<div
					class="row"
					class:selected={app.selected?.id === t.id}
					role="option"
					aria-selected={app.selected?.id === t.id}
					tabindex="-1"
					onpointerenter={() => (hovered = t.id)}
					onpointerleave={() => (hovered = null)}
				>
					<button class="row-main" onclick={() => app.select(t.id)}>
						<span class="title">{t.title}</span>
						<span class="meta"
							>{dur(t.duration)} · {compactModel(t.modelId)} · {shortDate(t.createdAt)}</span
						>
					</button>
					{#if hovered === t.id || app.selected?.id === t.id}
						<button
							class="trash"
							title="Delete this transcript"
							aria-label="Delete {t.title}"
							onclick={() => app.deleteWithConfirm(t.id, t.title)}
						>
							<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
								stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
								<path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
								<line x1="10" y1="11" x2="10" y2="17" />
								<line x1="14" y1="11" x2="14" y2="17" />
							</svg>
						</button>
					{/if}
				</div>
			{/each}
		{/each}
	</div>

	<footer class="count">
		{app.transcripts.length}
		{app.transcripts.length === 1 ? 'transcript' : 'transcripts'}
	</footer>
</aside>

<style>
	.sidebar {
		width: 260px;
		flex: none;
		display: flex;
		flex-direction: column;
		background: var(--surface-sidebar);
		border-right: 1px solid var(--border);
		overflow: hidden;
	}

	.traffic-spacer {
		height: 38px;
		flex: none;
	}

	.search {
		display: flex;
		align-items: center;
		gap: 7px;
		margin: 0 12px 10px;
		padding: 6px 10px;
		background: var(--surface-elevated);
		border: 1px solid var(--border);
		border-radius: 8px;
		color: var(--label-secondary);
	}

	.search input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		font-family: var(--font);
		font-size: 13px;
		color: var(--label-primary);
	}

	.search input::placeholder {
		color: var(--label-tertiary);
	}

	.list {
		flex: 1;
		overflow-y: auto;
		padding: 0 8px 8px;
	}

	.group-label {
		margin: 10px 8px 4px;
		font-size: 11px;
		font-weight: 600;
		color: var(--label-tertiary);
	}

	.empty {
		margin: 24px 12px;
		font-size: 12px;
		line-height: 1.5;
		color: var(--label-tertiary);
		text-align: center;
	}

	.row {
		position: relative;
		display: flex;
		align-items: center;
		border-radius: 5px;
		transition: background var(--t-hover);
	}

	.row:hover {
		background: var(--surface-highlight);
	}

	.row.selected {
		background: var(--accent-selection);
	}

	.row-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 5px 8px;
		background: none;
		border: none;
		text-align: left;
		cursor: pointer;
		color: inherit;
	}

	.title {
		font-size: 13px;
		font-weight: 500;
		color: var(--label-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.meta {
		font-size: 11px;
		color: var(--label-secondary);
	}

	.trash {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		margin-right: 4px;
		background: none;
		border: none;
		border-radius: 5px;
		color: var(--label-secondary);
		cursor: pointer;
	}

	.trash:hover {
		color: var(--red);
		background: var(--surface-highlight);
	}

	.count {
		flex: none;
		padding: 8px 12px;
		border-top: 1px solid var(--border);
		font-size: 11px;
		color: var(--label-secondary);
		font-variant-numeric: tabular-nums;
	}
</style>
