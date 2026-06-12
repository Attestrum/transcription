<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let info = $state<{ name: string; version: string } | null>(null);

  $effect(() => {
    invoke<{ name: string; version: string }>("product_info").then((v) => {
      info = v;
    });
  });
</script>

<main class="boot">
  <div class="wordmark">
    <span class="brand">ATTESTRUM</span>
    <span class="sep">▸</span>
    <span class="product">TRANSCRIPTION</span>
  </div>
  <div class="status">
    {#if info}
      SHELL v{info.version} · ENGINE PENDING<span class="cursor">▮</span>
    {:else}
      BOOTING<span class="cursor">▮</span>
    {/if}
  </div>
</main>

<style>
  .boot {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
  }

  .wordmark {
    font-size: 22px;
    letter-spacing: 0.32em;
  }

  .brand {
    color: var(--cyan);
    text-shadow: var(--glow-cyan);
  }

  .sep {
    color: var(--text-dim);
    margin: 0 6px;
  }

  .product {
    color: var(--green);
    text-shadow: var(--glow-green);
  }

  .status {
    font-size: 12px;
    letter-spacing: 0.22em;
    color: var(--text-dim);
  }

  .cursor {
    color: var(--green);
    margin-left: 6px;
    animation: cursor-blink 1.2s step-end infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .cursor {
      animation: none;
    }
  }
</style>
