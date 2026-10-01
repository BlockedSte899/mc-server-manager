<script lang="ts">
  interface Props {
    label: string;
    checked?: boolean;
    disabled?: boolean;
    onChange?: (checked: boolean) => void;
    hint?: string;
  }

  let { label, checked = $bindable(false), disabled = false, onChange, hint }: Props = $props();
</script>

<label class="flex items-start gap-3 cursor-pointer select-none group {disabled ? 'opacity-40 pointer-events-none' : ''}">
  <button
    type="button"
    aria-label={label}
    onclick={() => {
      checked = !checked;
      onChange?.(checked);
    }}
    class="relative mt-0.5 w-10 h-5 rounded-full transition-colors shrink-0 {checked ? 'bg-brand-500' : 'bg-surface-3'}"
  >
    <span
      class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {checked ? 'translate-x-5' : ''}"
    ></span>
  </button>
  <span>
    <span class="block text-sm text-slate-100">{label}</span>
    {#if hint}
      <span class="block text-xs text-fg-dim mt-0.5">{hint}</span>
    {/if}
  </span>
</label>