<script lang="ts">
	import { getCurrentWebview } from '@tauri-apps/api/webview';
	import { productInfo, type ProductInfo } from '$lib/api';
	import { app, IMPORT_EXTENSIONS } from '$lib/app-state.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import LibraryPane from '$lib/components/LibraryPane.svelte';
	import EditorPane from '$lib/components/EditorPane.svelte';
	import RecordMode from '$lib/components/RecordMode.svelte';
	import TranscribeView from '$lib/components/TranscribeView.svelte';
	import ModelSheet from '$lib/components/ModelSheet.svelte';

	let info = $state<ProductInfo | null>(null);
	let dragOver = $state(false);

	$effect(() => {
		app.load();
		productInfo()
			.then((v) => (info = v))
			.catch(() => {});
		const unwireEvents = app.wireEvents().catch(() => () => {});
		// Dropping a media file anywhere on the window imports it.
		const undrop = getCurrentWebview()
			.onDragDropEvent((event) => {
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
			})
			.catch(() => () => {});
		return () => {
			unwireEvents.then((fn) => fn());
			undrop.then((fn) => fn());
		};
	});

	// The FX setting drives the scan-line overlay in tokens.css.
	$effect(() => {
		document.body.dataset.fx = app.fxEnabled ? 'on' : 'off';
	});
</script>

<div class="shell">
	<TopBar />
	<div class="panes">
		<LibraryPane />
		{#if app.phase.kind === 'recording'}
			<RecordMode />
		{:else if app.phase.kind === 'transcribing'}
			<TranscribeView />
		{:else}
			<EditorPane />
		{/if}
	</div>
	<footer class="statusbar">
		<button class="fx" onclick={() => app.toggleFx()} title="Toggle scan-lines and glow">
			FX [{app.fxEnabled ? 'ON' : 'OFF'}]
		</button>
		{#if app.lastError}
			<span class="error" role="alert">! {app.lastError.message}</span>
		{/if}
		<span class="version">{info ? `v${info.version}` : ''}</span>
	</footer>
	{#if dragOver}
		<div class="dropzone" aria-hidden="true">
			<span>⬇ DROP TO TRANSCRIBE</span>
		</div>
	{/if}
	<ModelSheet />
</div>

<style>
	.shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	.panes {
		flex: 1;
		display: flex;
		min-height: 0;
	}

	.statusbar {
		display: flex;
		align-items: center;
		gap: 16px;
		height: 28px;
		padding: 0 12px;
		border-top: 1px solid var(--hairline);
		background: var(--panel);
		font-size: 10px;
		letter-spacing: 0.16em;
		color: var(--text-dim);
	}

	.fx {
		padding: 0;
		background: none;
		border: none;
		font-family: var(--mono);
		font-size: 10px;
		letter-spacing: 0.16em;
		color: var(--text-dim);
		cursor: pointer;
		transition: color var(--t-fast) var(--ease);
	}

	.fx:hover {
		color: var(--cyan);
	}

	.error {
		color: var(--red);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.version {
		margin-left: auto;
	}

	.dropzone {
		position: fixed;
		inset: 0;
		z-index: 90;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(10, 14, 20, 0.75);
		border: 1px dashed var(--cyan-dim);
		font-size: 14px;
		letter-spacing: 0.3em;
		color: var(--cyan);
		text-shadow: var(--glow-cyan);
		pointer-events: none;
	}
</style>
