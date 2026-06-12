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

	function ts(secs: number): string {
		const t = Math.floor(secs * 1000);
		const h = Math.floor(t / 3_600_000);
		const m = Math.floor(t / 60_000) % 60;
		const s = Math.floor(t / 1000) % 60;
		const ms = t % 1000;
		const pad = (n: number, w = 2) => String(n).padStart(w, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}.${pad(ms, 3)}`;
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

	function commitTitle(e: Event) {
		app.rename((e.target as HTMLInputElement).value);
	}

	// Focus the segment textarea as soon as it renders.
	function autofocus(el: HTMLTextAreaElement) {
		el.focus();
		el.setSelectionRange(el.value.length, el.value.length);
	}
</script>

<section class="editor">
	{#if app.selected}
		{@const t = app.selected}
		<header class="doc-head">
			<input
				class="title"
				value={t.title}
				onchange={commitTitle}
				onkeydown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
				spellcheck="false"
				aria-label="Transcript title"
			/>
			<div class="meta-row">
				<div class="chips">
					<span class="chip">{t.modelId}</span>
					<span class="chip">{t.language.toUpperCase()}</span>
					<span class="chip"
						>{Math.floor(t.duration / 60)}:{String(Math.floor(t.duration % 60)).padStart(
							2,
							'0'
						)}</span
					>
					{#if t.audioRelativePath}<span class="chip green">WAV</span>{/if}
				</div>
				<div class="find">
					<span class="prompt">/</span>
					<input
						type="text"
						bind:value={find}
						placeholder="find"
						spellcheck="false"
						aria-label="Find in transcript"
					/>
					{#if find.trim()}
						<span class="count" class:none={matchCount === 0}>{matchCount}</span>
					{/if}
				</div>
			</div>
		</header>

		<div class="segments">
			{#each t.segments as seg (seg.id)}
				<div
					class="segment"
					class:playing={playingSegmentId === seg.id}
					class:hit={matches(seg.text)}
					class:dimmed={find.trim() && !matches(seg.text)}
				>
					<button
						class="stamp"
						disabled={!app.playerLoaded}
						title={app.playerLoaded ? 'Seek here' : 'No stored audio for this transcript'}
						onclick={() => app.seek(seg.start)}
					>
						[{ts(seg.start)}]
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
						<button class="text" onclick={() => beginEdit(seg.id, seg.text)}>
							{seg.text}
							{#if seg.originalText !== undefined}
								<span class="edited" title={`engine: ${seg.originalText}`}>·edited</span>
							{/if}
						</button>
					{/if}
				</div>
			{/each}
		</div>

		{#if app.playerLoaded}
			<PlayerBar />
		{/if}
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

	.title {
		width: 100%;
		margin: 0 0 8px;
		padding: 0;
		background: transparent;
		border: none;
		outline: none;
		font-family: var(--mono);
		font-size: 16px;
		font-weight: 600;
		letter-spacing: 0.04em;
		color: var(--text);
		caret-color: var(--cyan);
	}

	.title:focus {
		border-bottom: 1px solid var(--cyan-dim);
	}

	.meta-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
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

	.find {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.find .prompt {
		color: var(--cyan-dim);
		font-size: 12px;
	}

	.find input {
		width: 120px;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--hairline);
		outline: none;
		font-family: var(--mono);
		font-size: 11px;
		color: var(--text);
		caret-color: var(--cyan);
		padding: 2px 0;
	}

	.find input:focus {
		border-bottom-color: var(--cyan-dim);
	}

	.find .count {
		font-size: 10px;
		color: var(--green);
	}

	.find .count.none {
		color: var(--red);
	}

	.segments {
		flex: 1;
		overflow-y: auto;
		padding: 16px 24px;
	}

	.segment {
		display: flex;
		gap: 12px;
		padding: 6px 8px 6px 10px;
		line-height: 1.55;
		border-left: 2px solid transparent;
		transition:
			border-color var(--t-fast) var(--ease),
			background var(--t-fast) var(--ease),
			opacity var(--t-fast) var(--ease);
	}

	.segment.playing {
		border-left-color: var(--cyan);
		background: var(--cyan-wash);
	}

	.segment.hit {
		background: var(--cyan-wash);
	}

	.segment.dimmed {
		opacity: 0.35;
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

	.stamp:hover:not(:disabled) {
		text-shadow: var(--glow-green);
	}

	.stamp:disabled {
		color: var(--text-dim);
		cursor: default;
	}

	.text {
		flex: 1;
		padding: 0;
		background: none;
		border: none;
		font-family: var(--mono);
		font-size: 13px;
		color: var(--text);
		text-align: left;
		line-height: 1.55;
		cursor: text;
		user-select: text;
	}

	.edited {
		margin-left: 8px;
		font-size: 10px;
		color: var(--text-dim);
	}

	textarea {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--cyan-dim);
		outline: none;
		font-family: var(--mono);
		font-size: 13px;
		line-height: 1.55;
		color: var(--text);
		caret-color: var(--cyan);
		padding: 2px 6px;
		resize: none;
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
