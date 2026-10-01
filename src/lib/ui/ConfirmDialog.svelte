<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import Button from "./Button.svelte";
  import { AlertTriangle } from "lucide-svelte";
  import { t } from "../../lib/i18n.svelte";

  let {
    open = false,
    title = "Are you sure?",
    message = "",
    confirmLabel = "Delete",
    danger = true,
    onConfirm,
    onClose,
  }: {
    open?: boolean;
    title?: string;
    message?: string;
    confirmLabel?: string;
    danger?: boolean;
    onConfirm?: () => void;
    onClose?: () => void;
  } = $props();

  const titleText = $derived(title && title !== "Are you sure?" ? title : t("Are you sure?"));
  const confirmText = $derived(confirmLabel === "Delete" ? t("Delete") : confirmLabel);
</script>

<Dialog title={titleText} open={open} onClose={onClose}>
  <div class="flex flex-col gap-4">
    <div class="flex items-start gap-3">
      {#if danger}
        <div class="w-9 h-9 rounded-lg bg-red-500/15 border border-red-500/30 flex items-center justify-center shrink-0">
          <AlertTriangle size={16} class="text-red-400" />
        </div>
      {/if}
      <p class="text-sm text-slate-200 leading-relaxed whitespace-pre-wrap">{message}</p>
    </div>
    <div class="flex items-center justify-end gap-2 mt-1">
      <Button variant="outline" size="sm" onclick={onClose}>{t("Cancel")}</Button>
      <Button
        variant={danger ? "danger" : "primary"}
        size="sm"
        onclick={() => {
          onConfirm?.();
          onClose?.();
        }}
      >
        {confirmText}
      </Button>
    </div>
  </div>
</Dialog>