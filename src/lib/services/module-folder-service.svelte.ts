import { invoke } from '@tauri-apps/api/core';
import type { Problem } from './module-messages';

/** An older version of a module, kept beside the newest. */
export interface OlderVersion {
  version: string;
  file: string;
}

/** A module in the modules folder. Matches `ModuleSummary` in src-tauri/src/modules/folder.rs. */
export interface ModuleSummary {
  id: string;
  name: string;
  /** The short name for tabs and menus, such as "KJV". */
  abbreviation: string;
  /** "bible", "dictionary", and so on: which menus list it. */
  moduleType: string;
  /** Whether it has footnotes, so a Notes tab can be offered. */
  hasNotes: boolean;
  version: string;
  file: string;
  /** From the module itself, which its signed file hash covers. */
  description: string;
  language: string;
  license: string;
  /** The app carries this module. */
  builtIn: boolean;
  /** Its text is exactly a pinned edition the app knows. */
  textVerified: boolean;
  /** Older versions in the folder, newest first. */
  older: OlderVersion[];
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
  problems: Problem[];
  /** When the module list was last checked, in seconds since 1970. */
  catalogCheckedAt: number | null;
}

/**
 * The visible modules folder. Rust chooses it, installs and opens the
 * modules, and runs the folder picker, so this service never handles paths
 * except to display the folder's location.
 */
class ModuleFolderService {
  public status = $state<FolderStatus | undefined>(undefined);
  /** The last command's error, for `errorMessage` to describe. */
  public error = $state<unknown>(undefined);
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

  /**
   * Opens the system folder picker, titled `title`. Returns the picked
   * folder for display, or undefined if the reader cancelled. Rust keeps
   * the folder until `changeFolder`.
   */
  public async pickFolder(title: string): Promise<string | undefined> {
    try {
      this.error = undefined;
      return (
        (await invoke<string | null>('pick_modules_folder', { title })) ??
        undefined
      );
    } catch (err) {
      this.error = err;
      return undefined;
    }
  }

  /**
   * Switches to the picked folder, moving the current modules there if
   * `moveModules`, or leaving them where they are.
   */
  public async changeFolder(moveModules: boolean): Promise<void> {
    this.changing = true;
    try {
      await this.run(() =>
        invoke<FolderStatus>('change_modules_folder', { moveModules })
      );
    } finally {
      this.changing = false;
    }
  }

  /** Opens the modules folder in the system's file manager. */
  public async openFolder(): Promise<void> {
    try {
      await invoke('open_modules_folder');
    } catch (err) {
      this.error = err;
    }
  }

  /** Deletes an older version of a module, given its file name. */
  public async deleteVersion(file: string): Promise<void> {
    await this.run(() =>
      invoke<FolderStatus>('delete_module_version', { file })
    );
  }

  /** Takes a status from another command, such as a download. */
  public update(status: FolderStatus): void {
    this.status = status;
  }

  private async run(command: () => Promise<FolderStatus>): Promise<void> {
    try {
      this.status = await command();
      this.error = undefined;
    } catch (err) {
      this.error = err;
    }
  }
}

export const moduleFolderService = new ModuleFolderService();
