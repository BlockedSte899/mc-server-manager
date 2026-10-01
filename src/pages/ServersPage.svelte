<script lang="ts">
  import { servers, refreshServers, windowHidden } from "../lib/store.svelte";
  import ServerCard from "../components/ServerCard.svelte";
  import { navigate } from "../lib/router.svelte";
  import Button from "../lib/ui/Button.svelte";
  import { t, pl } from "../lib/i18n.svelte";
  import { Plus } from "lucide-svelte";
  import { onMount } from "svelte";

  onMount(() => {
    refreshServers();
    const int = setInterval(() => {
      if (!windowHidden()) void refreshServers();
    }, 4000);
    return () => clearInterval(int);
  });
</script>

<div class="max-w-7xl mx-auto px-5 py-6">
  <div class="flex items-center justify-between mb-5">
    <div>
      <h1 class="text-xl font-bold text-slate-50">{t("Servers")}</h1>
      <p class="text-sm text-fg-dim mt-0.5">
        {servers.length === 0 ? t("No servers yet — create your first one.") : pl(servers.length, ["1 сервер", "{n} сервера", "{n} серверов"], "{n} servers")}
      </p>
    </div>
    <div class="flex items-center gap-2">
      <Button onclick={() => navigate("/wizard")}>
        <Plus size={16} /> {t("New Server")}
      </Button>
      <Button variant="outline" onclick={() => navigate("/wizard?core=velocity")}>
        <Plus size={16} /> {t("Velocity Proxy")}
      </Button>
    </div>
  </div>

  {#if servers.length > 0}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each servers as srv (srv.meta.id)}
        <ServerCard server={srv} />
      {/each}
    </div>
  {:else}
    <div class="rounded-xl border border-dashed border-edge p-12 text-center bg-surface-1">
      <p class="text-fg-dim text-sm">{t("Empty workspace. Create a new Minecraft server or import a modpack/archive.")}</p>
    </div>
  {/if}
</div>