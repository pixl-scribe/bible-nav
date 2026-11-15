/**
 * Opens the Modules dialog from anywhere, such as "Get more modules…" in a
 * module menu, on the tab wanted.
 */
export type ModulesTab = 'installed' | 'download';

class ModulesDialogService {
  public dialog = $state<HTMLDialogElement | undefined>(undefined);
  public tab = $state<ModulesTab>('installed');

  public open(tab: ModulesTab = 'installed'): void {
    this.tab = tab;
    this.dialog?.showModal();
  }
}

export const modulesDialogService = new ModulesDialogService();
