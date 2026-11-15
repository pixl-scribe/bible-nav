import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  moduleFolderService,
  type FolderStatus,
} from './module-folder-service.svelte';
import { isCancelled, type Problem } from './module-messages';

/** A module the catalog offers. Matches `OfferSummary` in src-tauri/src/modules/downloads.rs. */
export interface Offer {
  id: string;
  version: string;
  name: string;
  description: string;
  language: string;
  license: string;
  /** The compressed download's size in bytes. */
  size: number;
  /** A new text of a pinned module, which only a newer Bible Nav can use. */
  needsNewerApp: boolean;
  installedVersion: string | null;
  relation: 'new' | 'installed' | 'update' | 'older';
  /** An update to a new major version: the text itself changed. */
  textChanges: boolean;
  textVerified: boolean;
}

interface CatalogStatus {
  offers: Offer[];
  problems: Problem[];
  checkedAt: number;
  folder: FolderStatus;
}

interface Progress {
  id: string;
  version: string;
  done: number;
  total: number;
}

/** A download in progress: bytes so far of the total. */
export interface DownloadState {
  done: number;
  total: number;
}

export function offerKey(offer: { id: string; version: string }): string {
  return `${offer.id}-${offer.version}`;
}

/**
 * The module catalog, read only when the reader asks (`check`), and the
 * downloads from it. Checking and downloading go online; nothing else here
 * does.
 */
class ModuleCatalogService {
  /** The offers from the last check this session; undefined until checked. */
  public offers = $state<Offer[] | undefined>(undefined);
  public problems = $state<Problem[]>([]);
  public checking = $state(false);
  /** The last check's error, for `errorMessage` to describe. */
  public error = $state<unknown>(undefined);
  /** Downloads in progress, by `offerKey`. */
  public downloads = $state<Record<string, DownloadState>>({});
  /** Failed downloads' errors, by `offerKey`. */
  public failures = $state<Record<string, unknown>>({});
  private listening = false;

  /** Reads the catalog. Goes online. */
  public async check(): Promise<void> {
    this.checking = true;
    try {
      const status = await invoke<CatalogStatus>('check_for_modules');
      this.offers = status.offers;
      this.problems = status.problems;
      this.error = undefined;
      moduleFolderService.update(status.folder);
    } catch (err) {
      this.error = err;
    } finally {
      this.checking = false;
    }
  }

  /** Downloads an offer. Goes online. */
  public async download(offer: Offer): Promise<void> {
    await this.listen();
    const key = offerKey(offer);
    this.downloads[key] = { done: 0, total: offer.size };
    delete this.failures[key];
    try {
      const folder = await invoke<FolderStatus>('download_module', {
        id: offer.id,
        version: offer.version,
      });
      moduleFolderService.update(folder);
      // The new version is now the newest installed one.
      this.offers = this.offers?.map((other) =>
        other.id === offer.id ? relateTo(other, offer.version) : other
      );
    } catch (err) {
      if (!isCancelled(err)) this.failures[key] = err;
    } finally {
      delete this.downloads[key];
    }
  }

  public async cancel(offer: Offer): Promise<void> {
    await invoke('cancel_module_download', {
      id: offer.id,
      version: offer.version,
    });
  }

  private async listen(): Promise<void> {
    if (this.listening) return;
    this.listening = true;
    await listen<Progress>('module-download-progress', ({ payload }) => {
      const key = offerKey(payload);
      if (this.downloads[key]) {
        this.downloads[key] = { done: payload.done, total: payload.total };
      }
    });
  }
}

/**
 * Compares versions like "1.10.0" and "1.9.2" by their numeric parts, as
 * `compare_versions` does in Rust.
 */
export function compareVersions(a: string, b: string): number {
  const left = a.split('.');
  const right = b.split('.');
  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const x = left[i] ?? '0';
    const y = right[i] ?? '0';
    const order =
      /^\d+$/.test(x) && /^\d+$/.test(y)
        ? Number(x) - Number(y)
        : x.localeCompare(y);
    if (order !== 0) return Math.sign(order);
  }
  return 0;
}

/** An offer's relation once `installed` is the newest installed version. */
export function relateTo(offer: Offer, installed: string): Offer {
  const order = compareVersions(offer.version, installed);
  const relation: Offer['relation'] =
    order > 0 ? 'update' : order === 0 ? 'installed' : 'older';
  const major = (version: string) => version.split('.')[0];
  return {
    ...offer,
    installedVersion: installed,
    relation,
    textChanges:
      relation === 'update' && major(offer.version) !== major(installed),
  };
}

export const moduleCatalogService = new ModuleCatalogService();
