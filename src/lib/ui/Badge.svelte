<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
    class?: string;
  }

  let { children, class: cls = "" }: Props = $props();
</script>

<span
  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium border {cls}"
>
  {@render children()}
</span>