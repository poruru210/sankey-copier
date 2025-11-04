import type { CopySettings } from '@/types';

/**
 * Find all receivers connected to a source account
 *
 * @param sourceId - The source account ID
 * @param settings - List of copy settings
 * @returns Array of receiver account IDs
 */
export function findConnectedReceivers(sourceId: string, settings: CopySettings[]): string[] {
  return settings
    .filter((s) => s.master_account === sourceId)
    .map((s) => s.slave_account);
}

/**
 * Find all sources connected to a receiver account
 *
 * @param receiverId - The receiver account ID
 * @param settings - List of copy settings
 * @returns Array of source account IDs
 */
export function findConnectedSources(receiverId: string, settings: CopySettings[]): string[] {
  return settings
    .filter((s) => s.slave_account === receiverId)
    .map((s) => s.master_account);
}

/**
 * Determine if an SVG path should be highlighted based on hover state
 *
 * @param path - The SVG path element
 * @param hoveredSourceId - Currently hovered source ID (if any)
 * @param hoveredReceiverId - Currently hovered receiver ID (if any)
 * @returns True if the path should be highlighted
 */
export function shouldHighlightPath(
  path: SVGPathElement,
  hoveredSourceId: string | null,
  hoveredReceiverId: string | null
): boolean {
  const sourceId = path.getAttribute('data-source-id');
  const receiverId = path.getAttribute('data-receiver-id');
  const receiverIdsStr = path.getAttribute('data-receiver-ids');
  const sourceIdsStr = path.getAttribute('data-source-ids');

  // Split and filter to avoid empty strings and 'null' string
  const receiverIds = receiverIdsStr && receiverIdsStr !== 'null'
    ? receiverIdsStr.split(',').filter(id => id.trim())
    : [];
  const sourceIds = sourceIdsStr && sourceIdsStr !== 'null'
    ? sourceIdsStr.split(',').filter(id => id.trim())
    : [];

  let shouldHighlight = false;

  // Check if hovering over a source
  if (hoveredSourceId) {
    // This is a line from source to server
    if (sourceId === hoveredSourceId) {
      shouldHighlight = true;
    }
    // This is a line from server to receiver - check if it connects to the hovered source
    else if (receiverId && sourceIds.includes(hoveredSourceId)) {
      shouldHighlight = true;
    }
  }

  // Check if hovering over a receiver
  if (hoveredReceiverId) {
    // This is a line from server to receiver
    if (receiverId === hoveredReceiverId) {
      shouldHighlight = true;
    }
    // This is a line from source to server - check if it connects to the hovered receiver
    else if (sourceId && receiverIds.includes(hoveredReceiverId)) {
      shouldHighlight = true;
    }
  }

  return shouldHighlight;
}
