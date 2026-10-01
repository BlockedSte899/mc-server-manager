<script lang="ts">
  import { toasts } from "../lib/store.svelte";
  import { CheckCircle2, Info, AlertTriangle } from "lucide-svelte";

  const icons = {
    info: Info,
    success: CheckCircle2,
    error: AlertTriangle,
  };
</script>

{#if toasts.length > 0}
  <div class="fixed bottom-4 right-4 z-[100] flex flex-col gap-2 max-w-sm">
    {#each toasts as t (t.id)}
      {@const Icon = icons[t.kind]}
      <div
        class="flex items-start gap-2 px-4 py-3 rounded-lg border shadow-lg bg-surface-2 backdrop-blur
          {t.kind === 'error' ? 'border-red-500/40' : t.kind === 'success' ? 'border-brand-500/40' : 'border-edge'}"
      >
        <Icon size={16} class="mt-0.5 shrink-0 {t.kind === 'error' ? 'text-red-400' : t.kind === 'success' ? 'text-brand-500' : 'text-accent'}" />
        <span class="text-sm text-slate-100">{t.message}</span>
      </div>
    {/each}
  </div>
{/if}