<script lang="ts">
  interface Props {
    value: number; // 0..100
    class?: string;
  }

  let { value, class: cls = "" }: Props = $props();
</script>

<div class="h-1.5 w-full bg-surface-3 rounded-full overflow-hidden {cls}">
  <div
    class="h-full bg-brand-500 rounded-full transition-all duration-200"
    style="width: {Math.min(100, Math.max(0, value))}%"
  ></div>
</div>