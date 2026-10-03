<script lang="ts">
  import { ChevronDown } from "lucide-svelte";

  interface Props {
    label?: string;
    value?: string;
    options: { value: string; label: string }[];
    class?: string;
    size?: "md" | "xs";
    disabled?: boolean;
    oninput?: (value: string) => void;
    onChange?: (value: string) => void;
  }

  let {
    label,
    value = $bindable(""),
    options,
    class: cls = "",
    size = "md",
    disabled = false,
    oninput,
    onChange,
  }: Props = $props();

  let open = $state(false);
  let rootEl = $state<HTMLElement | null>(null);
  let menuEl = $state<HTMLElement | null>(null);
  let pos = $state<{ top: number | null; bottom: number | null; left: number; width: number } | null>(null);

  const selected = $derived(options.find((o) => o.value === value));

  // The menu is `position: fixed`, i.e. positioned relative to the *viewport* —
  // exactly what getBoundingClientRect() reports. The native webview page zoom
  // (the primary scaling path) keeps these coordinates consistent, so the menu
  // opens right under (or above) the trigger regardless of zoom level.
  //
  // (An older version subtracted the #ui-root rect here; that was only valid
  // while a CSS transform on the wrapper made it the containing block for
  // fixed elements — under plain page zoom it made the menu drift down by the
  // scroll offset and open outside the window.)
  const MENU_MAX_H = 256;
  const rowH = 32; // px-3 py-1.5 text-sm ≈ 32px per row

  function place() {
    if (!rootEl) return;
    const r = rootEl.getBoundingClientRect();
    const h = Math.min(MENU_MAX_H, options.length * rowH + 8);
    const below = window.innerHeight - r.bottom - 8;
    const left = Math.max(8, Math.min(r.left, window.innerWidth - r.width - 8));
    if (below < h && r.top - 8 > below) {
      // not enough room below the trigger — open upward instead
      pos = { top: null, bottom: window.innerHeight - r.top + 4, left, width: r.width };
    } else {
      pos = { top: r.bottom + 4, bottom: null, left, width: r.width };
    }
  }

  function toggle() {
    place();
    open = !open;
  }

  function choose(v: string) {
    if (v === value) {
      open = false;
      return;
    }
    value = v;
    oninput?.(v);
    onChange?.(v);
    open = false;
  }

  $effect(() => {
    if (!open) return;
    function onDown(e: PointerEvent) {
      const t = e.target as Node;
      if (menuEl && menuEl.contains(t)) return;
      if (rootEl && !rootEl.contains(t)) open = false;
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") open = false;
    }
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  });
</script>

<div class="relative flex flex-col gap-1.5 {cls}" bind:this={rootEl}>
  {#if label}
    <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{label}</span>
  {/if}
  <button
    type="button"
    role="combobox"
    aria-expanded={open}
    aria-label={label}
    aria-haspopup="listbox"
    disabled={disabled}
    class="flex items-center justify-between gap-2 w-full text-left bg-surface-2 border border-edge rounded-lg text-slate-100 outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/40 transition cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed {size === 'xs' ? 'px-2 py-1 text-xs' : 'px-3 py-2 text-sm'}"
    onclick={toggle}
  >
    <span class="truncate">{selected?.label ?? "—"}</span>
    <ChevronDown size={size === "xs" ? 12 : 14} class="shrink-0 text-fg-dim transition-transform {open ? 'rotate-180' : ''}" />
  </button>
</div>

{#if open && pos}
  <ul
    role="listbox"
    bind:this={menuEl}
    class="fixed z-[100] py-1 rounded-lg border border-edge bg-surface-1 shadow-xl shadow-black/40 max-h-64 overflow-y-auto"
    style="top: {pos.top !== null ? `${pos.top}px` : "auto"}; bottom: {pos.bottom !== null ? `${pos.bottom}px` : "auto"}; left: {pos.left}px; width: {pos.width}px;"
  >
    {#each options as opt (opt.value)}
      <li
        role="option"
        aria-selected={opt.value === value}
        tabindex="-1"
        class="px-3 py-1.5 text-sm cursor-pointer whitespace-nowrap {opt.value === value ? 'bg-brand-500/15 text-brand-300' : 'text-slate-100 hover:bg-surface-2'}"
        onclick={() => choose(opt.value)}
      >
        {opt.label}
      </li>
    {/each}
  </ul>
{/if}