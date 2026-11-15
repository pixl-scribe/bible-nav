/**
 * What the Modules dialog shows for each module, worked out from the
 * folder status and the offers from the last check.
 */
import { compareVersions, type Offer } from './module-catalog-service.svelte';

/** The newest offer per module id, so each module is listed once. */
export function newestOffers(offers: Offer[]): Offer[] {
  const newest = new Map<string, Offer>();
  for (const offer of offers) {
    const current = newest.get(offer.id);
    if (!current || compareVersions(offer.version, current.version) > 0) {
      newest.set(offer.id, offer);
    }
  }
  return [...newest.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** The newest update this app can install for a module, if any. */
export function updateFor(id: string, offers: Offer[]): Offer | undefined {
  return newestOffers(
    offers.filter(
      (offer) =>
        offer.id === id && offer.relation === 'update' && !offer.needsNewerApp
    )
  )[0];
}

/**
 * The newest new text of a pinned module that only a newer Bible Nav can
 * use, if any.
 */
export function newerTextFor(id: string, offers: Offer[]): Offer | undefined {
  return newestOffers(
    offers.filter(
      (offer) =>
        offer.id === id && offer.relation === 'update' && offer.needsNewerApp
    )
  )[0];
}

/**
 * A license short enough for a summary line: a short license as it is, a
 * long one by its first sentence. The whole license is in Details.
 */
export function shortLicense(license: string): string {
  if (license.length <= 40) return license;
  const first = license.split(/(?<=\.)\s/)[0];
  return first.length <= 60
    ? first.replace(/\.$/, '')
    : `${license.slice(0, 57)}…`;
}
