<script lang="ts">
	import { app } from '../app-state.svelte';
	import PlayerBar from './PlayerBar.svelte';

	let find = $state('');
	let editingId = $state<number | null>(null);
	let draft = $state('');

	const playingSegmentId = $derived.by(() => {
		const t = app.selected;
		if (!t || !app.playerLoaded) return null;
		const pos = app.position;
		const seg = t.segments.find((s) => pos >= s.start && pos < s.end);
		return seg?.id ?? null;
	});

	const matchCount = $derived.by(() => {
		const t = app.selected;
		const q = find.trim().toLowerCase();
		if (!t || !q) return 0;
		return t.segments.filter((s) => s.text.toLowerCase().includes(q)).length;
	});

	function matches(text: string): boolean {
		const q = find.trim().toLowerCase();
		return q.length > 0 && text.toLowerCase().includes(q);
	}

	/** "M:SS" / "H:MM:SS" — the original's timestamp format. */
	function ts(secs: number): string {
		const t = Math.floor(secs);
		const h = Math.floor(t / 3600);
		const m = Math.floor((t % 3600) / 60);
		const s = t % 60;
		const pad = (n: number) => String(n).padStart(2, '0');
		return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
	}

	function beginEdit(id: number, text: string) {
		editingId = id;
		draft = text;
	}

	function commitEdit() {
		if (editingId === null) return;
		const id = editingId;
		editingId = null;
		app.editSegment(id, draft);
	}

	function editKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			commitEdit();
		} else if (e.key === 'Escape') {
			editingId = null;
		}
	}

	function autofocus(el: HTMLTextAreaElement) {
		el.focus();
		el.setSelectionRange(el.value.length, el.value.length);
	}
</script>

{#if app.selected}
	{@const t = app.selected}
	<div class="editor">
		<header class="head">
			<input
				class="title"
				value={t.title}
				onchange={(e) => app.rename((e.target as HTMLInputElement).value)}
				onkeydown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
				spellcheck="false"
				title="Click to rename — used as the default filename when exporting"
				aria-label="Transcript title"
			/>
			<div class="chips">
				<span class="chip" title="Duration">
					<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
						<circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 3" stroke-linecap="round" />
					</svg>
					{ts(t.duration)}
				</span>
				<span class="chip" title="Model">
					<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
						<line x1="4" y1="10" x2="4" y2="14" /><line x1="9" y1="6" x2="9" y2="18" />
						<line x1="14" y1="3" x2="14" y2="21" /><line x1="19" y1="8" x2="19" y2="16" />
					</svg>
					{t.modelId}
				</span>
				<span class="chip" title="Language">{t.language.toUpperCase()}</span>
				<span class="chip" title="Created">
					<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
						<rect x="3" y="5" width="18" height="16" rx="2" /><line x1="3" y1="10" x2="21" y2="10" />
						<line x1="8" y1="3" x2="8" y2="7" stroke-linecap="round" /><line x1="16" y1="3" x2="16" y2="7" stroke-linecap="round" />
					</svg>
					{new Date(t.createdAt).getMonth() + 1}/{new Date(t.createdAt).getDate()}/{String(
						new Date(t.createdAt).getFullYear()
					).slice(2)}
				</span>
				<span class="chip find" class:focused={find.length > 0}>
					<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true">
						<circle cx="11" cy="11" r="7" /><line x1="21" y1="21" x2="16.5" y2="16.5" />
					</svg>
					<input bind:value={find} placeholder="Find" spellcheck="false" aria-label="Find in transcript" />
					{#if find.trim()}
						<span class="counter" class:none={matchCount === 0}>{matchCount}</span>
					{/if}
				</span>
			</div>
		</header>

		<div class="segments">
			{#each t.segments as seg (seg.id)}
				<div class="segment" class:playing={playingSegmentId === seg.id}>
					<button
						class="stamp"
						class:active={playingSegmentId === seg.id}
						disabled={!app.playerLoaded}
						title={app.playerLoaded ? 'Click to seek' : 'No stored audio for this transcript'}
						onclick={() => app.seek(seg.start)}
					>
						{ts(seg.start)}
					</button>
					{#if editingId === seg.id}
						<textarea
							bind:value={draft}
							onblur={commitEdit}
							onkeydown={editKeydown}
							use:autofocus
							rows={Math.max(1, Math.ceil(draft.length / 80))}
							aria-label="Edit segment text"
						></textarea>
					{:else}
						<button
							class="text"
							class:hit={matches(seg.text)}
							title={seg.originalText !== undefined ? `Original: ${seg.originalText}` : undefined}
							onclick={() => beginEdit(seg.id, seg.text)}
						>
							{seg.text}
						</button>
					{/if}
				</div>
			{/each}
		</div>

		{#if app.playerLoaded}
			<PlayerBar />
		{/if}
	</div>
{:else}
	<!-- Home state is rendered by the page (DropZone). -->
{/if}

<style>
	.editor {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-height: 0;
	}

	.head {
		padding: 4px 20px 12px;
		border-bottom: 1px solid var(--border);
	}

	.title {
		width: 100%;
		margin: 0 0 10px;
		padding: 0;
		background: transparent;
		border: none;
		outline: none;
		font-family: var(--font);
		font-size: 16px;
		font-weight: 600;
		color: var(--label-primary);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 3px 8px;
		background: var(--surface-elevated);
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		font-size: 11px;
		font-weight: 500;
		color: var(--label-secondary);
		white-space: nowrap;
	}

	.chip.find input {
		width: 60px;
		background: transparent;
		border: none;
		outline: none;
		font-family: var(--font);
		font-size: 11px;
		color: var(--label-primary);
	}

	.chip.find input::placeholder {
		color: var(--label-tertiary);
	}

	.chip.find.focused {
		border-color: var(--accent-focus);
	}

	.counter {
		font-variant-numeric: tabular-nums;
		color: var(--label-secondary);
	}

	.counter.none {
		color: var(--label-tertiary);
	}

	.segments {
		flex: 1;
		overflow-y: auto;
		padding: 12px 16px;
	}

	.segment {
		display: flex;
		gap: 10px;
		padding: 4px 6px;
		border-radius: 4px;
		line-height: 1.5;
	}

	.segment.playing {
		background: var(--active-segment);
	}

	.segment:hover:not(.playing) {
		background: var(--surface-elevated);
	}

	.stamp {
		flex: none;
		width: 64px;
		padding: 0;
		background: none;
		border: none;
		font-family: var(--mono);
		font-size: 11px;
		text-align: right;
		color: var(--label-tertiary);
		cursor: pointer;
		line-height: inherit;
	}

	.stamp.active {
		color: var(--accent);
	}

	.stamp:disabled {
		cursor: default;
	}

	.text {
		flex: 1;
		padding: 0;
		background: none;
		border: none;
		font-family: var(--font);
		font-size: 14px;
		line-height: 1.5;
		color: var(--label-primary);
		text-align: left;
		cursor: text;
		user-select: text;
	}

	.text.hit {
		background: var(--find-match);
		color: #000;
		border-radius: 2px;
	}

	textarea {
		flex: 1;
		background: var(--surface-elevated);
		border: 1px solid var(--accent-focus);
		border-radius: 4px;
		outline: none;
		font-family: var(--font);
		font-size: 14px;
		line-height: 1.5;
		color: var(--label-primary);
		padding: 2px 6px;
		resize: none;
	}
</style>
