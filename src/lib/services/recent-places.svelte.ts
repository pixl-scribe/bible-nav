/**
 * The places the reader went to with the reference picker, newest first,
 * shared by every search, for the picker's Recent row.
 *
 * Kept in memory only, for this run, and never written to disk: where
 * someone has been reading is the kind of trace a reader under persecution
 * must not leave behind, especially on a portable copy.
 */
const limit = 8;

class RecentPlaces {
  /** Verse sids, such as 'ROM 8:15', newest first. */
  public places = $state<string[]>([]);

  public add(sid: string): void {
    this.places = [sid, ...this.places.filter((place) => place !== sid)].slice(
      0,
      limit
    );
  }
}

export const recentPlaces = new RecentPlaces();
