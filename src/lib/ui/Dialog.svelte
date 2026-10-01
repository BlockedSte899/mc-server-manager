<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "lucide-svelte";
  import { t } from "../../lib/i18n.svelte";

  interface Props {
    title?: string;
    open?: boolean;
    wide?: boolean;
    onClose?: () => void;
    children: Snippet;
  }

  let { title, open = false, wide = false, onClose, children }: Props = $props();
</script>

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      role="button"
      tabindex="-1"
      aria-label={t("Close")}
      onclick={onClose}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") onClose?.();
      }}
    ></div>
    <div
      class="relative w-full {wide ? 'max-w-3xl' : 'max-w-lg'} max-h-[88vh] flex flex-col rounded-xl border border-edge bg-surface-1 shadow-2xl"
    >
      {#if title}
        <header class="flex items-center justify-between px-4 py-3 border-b border-edge">
          <h3 class="text-sm font-semibold">{title}</h3>
          <button class="text-fg-dim hover:text-white cursor-pointer" onclick={onClose}>
            <X size={16} />
          </button>
        </header>
      {/if}
      <div class="p-4 overflow-y-auto">{@render children()}</div>
    </div>
  </div>
{/if}