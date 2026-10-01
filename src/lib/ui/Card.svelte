<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title?: string;
    subtitle?: string;
    class?: string;
    bodyClass?: string;
    actions?: Snippet;
    children: Snippet;
  }

  let { title, subtitle, class: cls = "", bodyClass = "", actions, children }: Props = $props();
</script>

<section class="rounded-xl border border-edge bg-surface-1 overflow-hidden {cls}">
  {#if title}
    <header class="flex items-center justify-between px-4 py-3 border-b border-edge bg-surface-2/60">
      <div>
        <h3 class="text-sm font-semibold text-slate-100">{title}</h3>
        {#if subtitle}
          <p class="text-xs text-fg-dim mt-0.5">{subtitle}</p>
        {/if}
      </div>
      {#if actions}
        <div class="flex items-center gap-2">{@render actions()}</div>
      {/if}
    </header>
  {/if}
  <div class="p-4 {bodyClass}">{@render children()}</div>
</section>