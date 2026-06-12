<script lang="ts">
	import { app } from '../app-state.svelte';
</script>

<!-- The main pane's title row: app title left, mic + settings right
     (toolbar .primaryAction in the original). Doubles as window drag area. -->
<header class="titlebar" data-tauri-drag-region>
	<h1 data-tauri-drag-region>Attestrum Transcription</h1>
	<div class="actions">
		<button
			class="icon-btn"
			class:recording={app.phase.kind === 'recording'}
			disabled={app.phase.kind === 'transcribing'}
			title="Record"
			aria-label="Record"
			onclick={() => app.toggleRecord()}
		>
			<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
				stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<rect x="9" y="3" width="6" height="11" rx="3" />
				<path d="M5 11a7 7 0 0 0 14 0" />
				<line x1="12" y1="18" x2="12" y2="21" />
			</svg>
		</button>
		<button class="icon-btn" disabled title="Settings (coming soon)" aria-label="Settings">
			<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
				stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<circle cx="12" cy="12" r="3" />
				<path
					d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
			</svg>
		</button>
	</div>
</header>

<style>
	.titlebar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 52px;
		padding: 0 16px 0 20px;
		flex: none;
	}

	h1 {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		letter-spacing: 0;
		pointer-events: none;
	}

	.actions {
		display: flex;
		gap: 4px;
		background: var(--surface-elevated);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 2px;
	}

	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: none;
		border-radius: 999px;
		background: transparent;
		color: var(--label-primary);
		cursor: pointer;
		transition: background var(--t-hover);
	}

	.icon-btn:hover:not(:disabled) {
		background: var(--surface-highlight);
	}

	.icon-btn:disabled {
		color: var(--label-quaternary);
		cursor: default;
	}

	.icon-btn.recording {
		color: var(--red);
	}
</style>
