<script lang="ts">
  import { t } from "../../lib/i18n.svelte";

  let { class: cls = "" }: { class?: string } = $props();
</script>

<div
  class="inline-block w-5 h-5 rounded-full border-2 border-gray-400/30 border-t-brand-500 animate-spin {cls}"
  aria-label={t("loading")}
></div>