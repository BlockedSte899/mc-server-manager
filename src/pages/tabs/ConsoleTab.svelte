<script lang="ts">
  import Console from "../../components/Console.svelte";

  let { id }: { id: string } = $props();
</script>

<Console serverId={id} height="560px" />