<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    value: string;
    sub?: string;
    class?: string;
    icon?: Snippet;
  }

  let { label, value, sub, class: cls = "", icon }: Props = $props();
</script>

<div class="rounded-xl border border-edge bg-surface-1 p-4 {cls}">
  <div class="flex items-center justify-between">
    <span class="text-xs font-medium uppercase tracking-wide text-fg-dim">{label}</span>
    {#if icon}
      <span class="text-brand-500">{@render icon()}</span>
    {/if}
  </div>
  <div class="mt-2 text-2xl font-bold text-slate-50 font-mono">{value}</div>
  {#if sub}
    <div class="mt-1 text-xs text-fg-dim">{sub}</div>
  {/if}
</div>