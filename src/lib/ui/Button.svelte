<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    variant?: "primary" | "outline" | "ghost" | "danger" | "subtle";
    size?: "xs" | "sm" | "md" | "icon";
    disabled?: boolean;
    type?: "button" | "submit";
    class?: string;
    title?: string;
    ariaLabel?: string;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let { variant = "primary", size = "md", disabled = false, type = "button", class: cls = "", title, ariaLabel, onclick, children }: Props = $props();

  const variantCls: Record<string, string> = {
    primary:
      "bg-brand-500 hover:bg-brand-600 text-slate-950 font-semibold shadow-md shadow-brand-500/20 border border-brand-500",
    outline:
      "border border-edge bg-surface-2 hover:bg-surface-3 text-slate-100",
    ghost: "hover:bg-surface-2 text-slate-200",
    danger: "bg-red-600 hover:bg-red-500 text-white border border-red-500/50",
    subtle:
      "bg-surface-2 hover:bg-surface-3 text-slate-200 border border-edge",
  };
  const sizeCls: Record<string, string> = {
    xs: "text-xs px-2 py-1 rounded-md gap-1",
    sm: "text-sm px-3 py-1.5 rounded-lg gap-1.5",
    md: "text-sm px-4 py-2 rounded-lg gap-2",
    icon: "p-2 rounded-lg",
  };
</script>

<button
  {type}
  {disabled}
  {title}
  aria-label={ariaLabel}
  onclick={onclick}
  class="inline-flex items-center justify-center transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer select-none whitespace-nowrap {variantCls[variant]} {sizeCls[size]} {cls}"
>
  {@render children()}
</button>