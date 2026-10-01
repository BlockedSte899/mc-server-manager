<script lang="ts">
  interface Props {
    label: string;
    type?: string;
    value?: string;
    placeholder?: string;
    class?: string;
    oninput?: (value: string) => void;
    onChange?: (value: string) => void;
    onkeydown?: (e: KeyboardEvent) => void;
  }

  let { label, type = "text", value = $bindable(""), placeholder, class: cls = "", oninput, onChange, onkeydown }: Props = $props();
</script>

<div class="flex flex-col gap-1.5 {cls}">
  <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{label}</span>
  <input
    {type}
    bind:value
    {placeholder}
    aria-label={label}
    class="bg-surface-2 border border-edge rounded-lg px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/40 transition"
    oninput={(e) => oninput?.((e.currentTarget as HTMLInputElement).value)}
    onchange={(e) => onChange?.((e.currentTarget as HTMLInputElement).value)}
    onkeydown={onkeydown}
  />
</div>