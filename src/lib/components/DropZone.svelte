<script lang="ts">
	import { app } from '../app-state.svelte';

	interface Props {
		dragOver?: boolean;
	}
	let { dragOver = false }: Props = $props();
</script>

<!-- The home state: ImportDropZone ported — dashed 18px-radius border,
     waveform-plus glyph, click anywhere to browse. -->
<button class="zone" class:over={dragOver} onclick={() => app.importFile()}>
	<svg class="glyph" width="56" height="56" viewBox="0 0 56 56" fill="none"
		stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true">
		<line x1="10" y1="22" x2="10" y2="34" />
		<line x1="18" y1="16" x2="18" y2="40" />
		<line x1="26" y1="10" x2="26" y2="46" />
		<line x1="34" y1="18" x2="34" y2="38" />
		<circle cx="43" cy="38" r="9" fill="var(--surface-base)" />
		<line x1="43" y1="33.5" x2="43" y2="42.5" />
		<line x1="38.5" y1="38" x2="47.5" y2="38" />
	</svg>
	<span class="main">Drop an audio file to transcribe</span>
	<span class="sub"><span class="dim">or</span> <span class="link">click anywhere to browse</span></span>
	<span class="formats">mp3 · wav · m4a · mp4 · mov · ogg · flac · mkv</span>
</button>

<style>
	.zone {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		margin: 0 40px 40px;
		background: transparent;
		border: 2px dashed var(--label-quaternary);
		border-radius: 18px;
		cursor: pointer;
		transition:
			border-color var(--t-hover),
			color var(--t-hover);
	}

	.zone:hover,
	.zone.over {
		border-color: var(--accent);
	}

	.glyph {
		color: var(--label-secondary);
		margin-bottom: 4px;
		transition: color var(--t-hover);
	}

	.zone:hover .glyph,
	.zone.over .glyph {
		color: var(--accent);
	}

	.main {
		font-size: 16px;
		font-weight: 400;
		color: var(--label-primary);
	}

	.sub {
		font-size: 12px;
	}

	.dim {
		color: var(--label-secondary);
	}

	.link {
		color: var(--accent);
		text-decoration: underline;
	}

	.formats {
		padding-top: 6px;
		font-size: 11px;
		color: var(--label-tertiary);
	}
</style>
