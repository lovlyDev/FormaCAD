// Window-local source reference. It is neither persisted nor a writer grant.
// Project ID and the actual invoking window are checked again by the host.
export interface FolderSourceRef {
  readonly sourceId: string;
  readonly commitId: string;
}
