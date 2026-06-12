<script lang="ts">
	import { onPlaybackPosition, productInfo, type ProductInfo } from '$lib/api';
	import { app } from '$lib/app-state.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import LibraryPane from '$lib/components/LibraryPane.svelte';
	import EditorPane from '$lib/components/EditorPane.svelte';

	let info = $state<ProductInfo | null>(null);

	$effect(() => {
		app.load();
		productInfo()
			.then((v) => (info = v))
			.catch(() => {});
		const unlisten = onPlaybackPosition((p) => {
			app.position = p.secs;
			app.playing = p.playing;
		});
		return () => {
			unlisten.then((fn) => fn()).catch(() => {});
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
		<EditorPane />
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
</style>
