<script lang="ts">
  /**
   * Changing the modules folder: the system folder picker first, then a
   * choice to move the current modules there or leave them where they are.
   */
  import { _ } from 'svelte-i18n';
  import { moduleFolderService } from '$lib/services/module-folder-service.svelte';

  let dialog = $state<HTMLDialogElement>();
  let picked = $state<string | undefined>();
  let moveModules = $state(true);

  /** Opens the folder picker, then this dialog if a folder was picked. */
  export async function start() {
    picked = await moduleFolderService.pickFolder(
      $_('modules.change.picker-title')
    );
    if (picked) {
      moveModules = true;
      dialog?.showModal();
    }
  }

  async function confirm() {
    dialog?.close();
    await moduleFolderService.changeFolder(moveModules);
  }

  const choices = [
    {
      value: true,
      label: 'modules.change.move',
      hint: 'modules.change.move-hint',
    },
    {
      value: false,
      label: 'modules.change.leave',
      hint: 'modules.change.leave-hint',
    },
  ];
</script>

<dialog bind:this={dialog} class="modal" aria-labelledby="change-folder-title">
  <div class="modal-box max-w-lg flex flex-col gap-4">
    <div class="flex flex-col gap-1">
      <h3 id="change-folder-title" class="text-lg font-bold">
        {$_('modules.change.title')}
      </h3>
      <p class="text-sm text-base-content/70 break-all">{picked ?? ''}</p>
    </div>

    <fieldset class="flex flex-col gap-2">
      <legend class="text-sm font-semibold mb-2">
        {$_('modules.change.question')}
      </legend>
      {#each choices as choice (choice.label)}
        <label
          class="flex gap-3 items-start rounded-box p-3 cursor-pointer {moveModules ===
          choice.value
            ? 'bg-primary/15 outline outline-primary'
            : 'bg-base-200'}"
        >
          <input
            type="radio"
            class="radio radio-primary radio-sm mt-1"
            name="move-modules"
            checked={moveModules === choice.value}
            onchange={() => (moveModules = choice.value)}
          />
          <span class="flex flex-col gap-1">
            <span class="text-sm font-semibold">{$_(choice.label)}</span>
            <span class="text-sm text-base-content/80">{$_(choice.hint)}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="modal-action mt-0">
      <form method="dialog">
        <button class="btn btn-outline">{$_('modules.change.cancel')}</button>
      </form>
      <button class="btn btn-primary" onclick={confirm}>
        {$_('modules.change.confirm')}
      </button>
    </div>
  </div>
</dialog>
