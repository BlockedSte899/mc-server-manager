<script lang="ts">
  interface Props {
    label?: string;
    value?: string;
    placeholder?: string;
    class?: string;
  }

  let { label, value = $bindable(""), placeholder, class: cls = "" }: Props = $props();
</script>

<div class="flex flex-col gap-1.5 {cls}">
  {#if label}
    <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{label}</span>
  {/if}
  <textarea
    bind:value
    {placeholder}
    aria-label={label}
    rows="3"
    class="bg-surface-2 border border-edge rounded-lg px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/40 transition resize-y"
  ></textarea>
</div>