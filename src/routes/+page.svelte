<script lang="ts">
	import { getCurrentWebview } from '@tauri-apps/api/webview';
	import { app, IMPORT_EXTENSIONS } from '$lib/app-state.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import LibraryPane from '$lib/components/LibraryPane.svelte';
	import EditorPane from '$lib/components/EditorPane.svelte';
	import DropZone from '$lib/components/DropZone.svelte';
	import RecordMode from '$lib/components/RecordMode.svelte';
	import TranscribeView from '$lib/components/TranscribeView.svelte';
	import ModelSheet from '$lib/components/ModelSheet.svelte';

	let dragOver = $state(false);

	$effect(() => {
		app.load();
		const unwireEvents = app.wireEvents().catch(() => () => {});
		// Dropping a media file anywhere on the window imports it.
		// getCurrentWebview() throws outside Tauri (plain-browser dev).
		const webview = (() => {
			try {
				return getCurrentWebview();
			} catch {
				return null;
			}
		})();
		const undrop = (webview?.onDragDropEvent((event) => {
				if (event.payload.type === 'over') {
					dragOver = app.phase.kind === 'idle';
				} else if (event.payload.type === 'drop') {
					dragOver = false;
					const media = event.payload.paths.find((p) =>
						IMPORT_EXTENSIONS.includes(p.split('.').pop()?.toLowerCase() ?? '')
					);
					if (media && app.phase.kind === 'idle') {
						app.transcribeSource({ type: 'file', path: media });
					}
				} else {
					dragOver = false;
				}
			}) ?? Promise.resolve(() => {})).catch(() => () => {});
		return () => {
			unwireEvents.then((fn) => fn());
			undrop.then((fn) => fn());
		};
	});
</script>

<div class="shell">
	<LibraryPane />
	<main class="main">
		<TopBar />
		{#if app.phase.kind === 'transcribing'}
			<TranscribeView />
		{:else if app.selected}
			<EditorPane />
		{:else}
			<DropZone {dragOver} />
		{/if}
	</main>

	{#if app.phase.kind === 'recording'}
		<RecordMode />
	{/if}
	<ModelSheet />

	{#if app.lastError}
		<div class="toast" role="alert">
			<span>{app.lastError.message}</span>
			<button onclick={() => (app.lastError = null)} aria-label="Dismiss">✕</button>
		</div>
	{/if}
</div>

<style>
	.shell {
		display: flex;
		height: 100vh;
	}

	.main {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--surface-base);
	}

	.toast {
		position: fixed;
		right: 16px;
		bottom: 16px;
		z-index: 110;
		display: flex;
		align-items: center;
		gap: 12px;
		max-width: 420px;
		padding: 10px 14px;
		background: var(--surface-sidebar);
		border: 1px solid var(--border-strong);
		border-radius: 10px;
		box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
		font-size: 12px;
		color: var(--label-primary);
	}

	.toast button {
		background: none;
		border: none;
		color: var(--label-secondary);
		font-size: 12px;
		cursor: pointer;
		padding: 2px;
	}

	.toast button:hover {
		color: var(--label-primary);
	}
</style>
