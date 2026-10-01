<script lang="ts">
  let { data, height = 64, max = 100, color = "var(--color-brand-500)", label, suffix = "" }: {
    data: number[];
    height?: number;
    max?: number;
    color?: string;
    label?: string;
    suffix?: string;
  } = $props();

  const W = 360;
  let points = $derived(
    data.length > 1
      ? data
          .map((v, i) => {
            const x = (i / (data.length - 1)) * W;
            const y = height - 4 - Math.min(1, v / max) * (height - 8);
            return `${x.toFixed(2)},${y.toFixed(2)}`;
          })
          .join(" ")
      : ""
  );
  let last = $derived(data.length ? data[data.length - 1] : null);
</script>

<div>
  {#if label}
    <div class="flex items-center justify-between mb-1">
      <span class="text-xs font-medium uppercase tracking-wide text-fg-dim">{label}</span>
      <span class="text-sm font-semibold text-slate-100">{last?.toFixed(1) ?? "—"}{suffix}</span>
    </div>
  {/if}
  <svg
    viewBox="0 0 {W} {height}"
    preserveAspectRatio="none"
    class="w-full"
    style="height:{height}px"
  >
    {#if points}
      <polyline points={points} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" />
      {#if last !== null}
        <circle
          cx={(data.length - 1) / (data.length - 1) * W}
          cy={height - 4 - Math.min(1, last / max) * (height - 8)}
          r="3"
          fill={color}
        />
      {/if}
    {:else}
      <line x1="0" y1={height / 2} x2={W} y2={height / 2} stroke="rgba(143,163,191,.2)" stroke-width="1" />
    {/if}
  </svg>
</div>