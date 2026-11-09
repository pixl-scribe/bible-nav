import { invoke } from '@tauri-apps/api/core';

/** A module found in the modules folder. */
export interface ModuleSummary {
  id: string;
  name: string;
  version: string;
  file: string;
}

/** Where modules are kept. Matches `FolderStatus` in src-tauri/src/modules/folder.rs. */
export interface FolderStatus {
  folder: string | null;
  /** Modules are next to the app, as on a thumb drive, so the folder can't change. */
  portable: boolean;
  /** Show the first-run notice about where modules are stored. */
  showNotice: boolean;
  modules: ModuleSummary[];
  /** Problems to show the reader, such as files that aren't valid modules. */
  problems: string[];
}

/**
 * The visible modules folder. Rust chooses it, installs and opens the
 * modules, and runs the folder picker, so this service never handles paths
 * except to display the folder's location.
 */
class ModuleFolderService {
  public status = $state<FolderStatus | undefined>(undefined);
  public error = $state<string | undefined>(undefined);
  public changing = $state(false);
  private rescanning = false;

  public async load(): Promise<void> {
    await this.run(() => invoke<FolderStatus>('modules_folder_status'));
  }

  public async dismissNotice(): Promise<void> {
    await this.run(() => invoke<FolderStatus>('dismiss_modules_folder_notice'));
  }

  /**
   * Looks for modules again, so ones copied into the folder while the app is
   * open appear. Called whenever the window regains focus.
   */
  public async rescan(): Promise<void> {
    if (this.rescanning || this.changing) return;
    this.rescanning = true;
    try {
      await this.run(() => invoke<FolderStatus>('rescan_modules'));
    } finally {
      this.rescanning = false;
    }
  }

  /** Opens the system folder picker, then moves the modules there. */
  public async changeFolder(): Promise<void> {
    this.changing = true;
    try {
      // Rust returns null if the reader cancels the picker.
      await this.run(() =>
        invoke<FolderStatus | null>('change_modules_folder')
      );
    } finally {
      this.changing = false;
    }
  }

  private async run(
    command: () => Promise<FolderStatus | null>
  ): Promise<void> {
    try {
      const status = await command();
      if (status) {
        this.status = status;
      }
      this.error = undefined;
    } catch (err) {
      this.error = String(err);
    }
  }
}

export const moduleFolderService = new ModuleFolderService();
