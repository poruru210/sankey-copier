import type { AccountInfo } from '@/types';

const SVG_NS = 'http://www.w3.org/2000/svg';
const CORNER_RADIUS = 8;
const STROKE_COLOR = '#d1d5db';
const STROKE_WIDTH = '2';
const SERVER_RADIUS = 20;
const SERVER_OFFSET = 20;
const VERTICAL_OFFSET = 30;

// Responsive breakpoints (matches Tailwind defaults)
const MOBILE_BREAKPOINT = 768; // md breakpoint

/**
 * Check if current viewport is mobile
 */
export function isMobileViewport(): boolean {
  if (typeof window === 'undefined') return false;
  return window.innerWidth < MOBILE_BREAKPOINT;
}

/**
 * Calculate the center position for the relay server icon
 */
export function calculateCenterPosition(
  sourceRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>,
  receiverRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>,
  sourceAccounts: AccountInfo[],
  receiverAccounts: AccountInfo[],
  svgRect: DOMRect,
  middleColumnRef?: React.RefObject<HTMLDivElement>
): { centerX: number; centerY: number } {
  const isMobile = isMobileViewport();

  // Use actual middle column position if available, otherwise fallback to center
  let centerX = svgRect.width / 2;
  if (!isMobile && middleColumnRef?.current) {
    const middleRect = middleColumnRef.current.getBoundingClientRect();
    centerX = middleRect.left - svgRect.left + middleRect.width / 2;
  }

  if (isMobile) {
    // Mobile: vertical layout - find the gap between source and receiver sections
    let sourceBottomY = 0;
    let receiverTopY = svgRect.height;

    // Find the bottom of the last source
    sourceAccounts.forEach((source) => {
      const sourceEl = sourceRefs.current[source.id];
      if (sourceEl) {
        const rect = sourceEl.getBoundingClientRect();
        const bottomY = rect.bottom - svgRect.top;
        if (bottomY > sourceBottomY) {
          sourceBottomY = bottomY;
        }
      }
    });

    // Find the top of the first receiver
    receiverAccounts.forEach((receiver) => {
      const receiverEl = receiverRefs.current[receiver.id];
      if (receiverEl) {
        const rect = receiverEl.getBoundingClientRect();
        const topY = rect.top - svgRect.top;
        if (topY < receiverTopY) {
          receiverTopY = topY;
        }
      }
    });

    // Center Y is the midpoint between source bottom and receiver top
    const centerY = (sourceBottomY + receiverTopY) / 2;

    return { centerX, centerY };
  } else {
    // Desktop: horizontal layout - average Y position of all accounts
    let totalY = 0;
    let count = 0;

    sourceAccounts.forEach((source) => {
      const sourceEl = sourceRefs.current[source.id];
      if (sourceEl) {
        const rect = sourceEl.getBoundingClientRect();
        totalY += rect.top - svgRect.top + rect.height / 2;
        count++;
      }
    });

    receiverAccounts.forEach((receiver) => {
      const receiverEl = receiverRefs.current[receiver.id];
      if (receiverEl) {
        const rect = receiverEl.getBoundingClientRect();
        totalY += rect.top - svgRect.top + rect.height / 2;
        count++;
      }
    });

    const centerY = count > 0 ? totalY / count : svgRect.height / 2;

    return { centerX, centerY };
  }
}

/**
 * Create SVG path for source to server connection (Desktop: horizontal layout)
 */
export function createSourceToServerPath(
  x1: number,
  y1: number,
  centerX: number,
  centerY: number
): string {
  const verticalX = centerX - VERTICAL_OFFSET;
  let pathData = `M ${x1} ${y1}`;

  if (Math.abs(y1 - centerY) > CORNER_RADIUS * 2) {
    // Horizontal line
    pathData += ` L ${verticalX - CORNER_RADIUS} ${y1}`;
    // Rounded corner
    if (y1 < centerY) {
      pathData += ` Q ${verticalX} ${y1}, ${verticalX} ${y1 + CORNER_RADIUS}`;
    } else {
      pathData += ` Q ${verticalX} ${y1}, ${verticalX} ${y1 - CORNER_RADIUS}`;
    }
    // Vertical line
    pathData += ` L ${verticalX} ${centerY - (y1 < centerY ? CORNER_RADIUS : -CORNER_RADIUS)}`;
    // Rounded corner to server
    pathData += ` Q ${verticalX} ${centerY}, ${verticalX + CORNER_RADIUS} ${centerY}`;
  } else {
    pathData += ` L ${verticalX} ${y1} L ${verticalX} ${centerY}`;
  }

  // Final horizontal to server
  pathData += ` L ${centerX - SERVER_OFFSET} ${centerY}`;

  return pathData;
}

/**
 * Create SVG path for source to server connection (Mobile: vertical layout)
 */
export function createSourceToServerPathMobile(
  x1: number,
  y1: number,
  centerX: number,
  centerY: number
): string {
  // Mobile: vertical connection from bottom of source card to top of server
  let pathData = `M ${x1} ${y1}`;

  // Straight vertical line to server
  const midY = centerY - SERVER_OFFSET;
  pathData += ` L ${x1} ${midY}`;

  return pathData;
}

/**
 * Create SVG path for server to receiver connection (Desktop: horizontal layout)
 */
export function createServerToReceiverPath(
  x2: number,
  y2: number,
  centerX: number,
  centerY: number
): string {
  const verticalX = centerX + VERTICAL_OFFSET;
  let pathData = `M ${centerX + SERVER_OFFSET} ${centerY}`;

  // Horizontal from server
  pathData += ` L ${verticalX - CORNER_RADIUS} ${centerY}`;

  if (Math.abs(y2 - centerY) > CORNER_RADIUS * 2) {
    // Rounded corner from horizontal to vertical
    if (y2 < centerY) {
      pathData += ` Q ${verticalX} ${centerY}, ${verticalX} ${centerY - CORNER_RADIUS}`;
    } else {
      pathData += ` Q ${verticalX} ${centerY}, ${verticalX} ${centerY + CORNER_RADIUS}`;
    }
    // Vertical line
    pathData += ` L ${verticalX} ${y2 - (y2 < centerY ? -CORNER_RADIUS : CORNER_RADIUS)}`;
    // Rounded corner to horizontal
    pathData += ` Q ${verticalX} ${y2}, ${verticalX + CORNER_RADIUS} ${y2}`;
  } else {
    pathData += ` L ${verticalX} ${centerY} L ${verticalX} ${y2}`;
  }

  // Final horizontal to receiver
  pathData += ` L ${x2} ${y2}`;

  return pathData;
}

/**
 * Create SVG path for server to receiver connection (Mobile: vertical layout)
 */
export function createServerToReceiverPathMobile(
  x2: number,
  y2: number,
  centerX: number,
  centerY: number
): string {
  // Mobile: vertical connection from bottom of server to top of receiver card
  let pathData = `M ${centerX} ${centerY + SERVER_OFFSET}`;

  // Straight vertical line to receiver
  pathData += ` L ${x2} ${y2}`;

  return pathData;
}

/**
 * Create SVG path element
 */
export function createPathElement(
  pathData: string,
  isActive: boolean,
  attributes: Record<string, string>
): SVGPathElement {
  const path = document.createElementNS(SVG_NS, 'path');
  path.setAttribute('d', pathData);
  path.setAttribute('stroke', STROKE_COLOR);
  path.setAttribute('stroke-width', STROKE_WIDTH);
  path.setAttribute('fill', 'none');

  // Add custom attributes
  Object.entries(attributes).forEach(([key, value]) => {
    path.setAttribute(key, value);
  });

  // Add dash array for inactive accounts
  if (!isActive) {
    path.setAttribute('stroke-dasharray', '5,5');
  }

  return path;
}

/**
 * Create relay server icon SVG group
 */
export function createRelayServerIcon(centerX: number, centerY: number, isMobile: boolean = false): SVGGElement {
  const serverGroup = document.createElementNS(SVG_NS, 'g');

  // Position the server icon
  // Desktop: use SERVER_OFFSET for horizontal offset
  // Mobile: center the icon at centerX, centerY
  const translateX = centerX - SERVER_RADIUS;
  const translateY = centerY - SERVER_RADIUS;

  serverGroup.setAttribute('transform', `translate(${translateX}, ${translateY})`);

  // Background circle
  const circle = document.createElementNS(SVG_NS, 'circle');
  circle.setAttribute('cx', String(SERVER_RADIUS));
  circle.setAttribute('cy', String(SERVER_RADIUS));
  circle.setAttribute('r', String(SERVER_RADIUS));
  circle.setAttribute('fill', '#3b82f6');
  serverGroup.appendChild(circle);

  // Server icon (3 horizontal bars)
  const rects = [
    { y: '12' },
    { y: '18' },
    { y: '24' },
  ];

  rects.forEach(({ y }) => {
    const rect = document.createElementNS(SVG_NS, 'rect');
    rect.setAttribute('x', '12');
    rect.setAttribute('y', y);
    rect.setAttribute('width', '16');
    rect.setAttribute('height', '4');
    rect.setAttribute('rx', '1');
    rect.setAttribute('fill', 'white');
    serverGroup.appendChild(rect);
  });

  return serverGroup;
}

/**
 * Update line opacity based on hover state
 */
export function updateLineOpacity(
  svg: SVGSVGElement,
  hoveredSourceId: string | null,
  hoveredReceiverId: string | null
): void {
  const paths = svg.querySelectorAll('path');

  paths.forEach((path) => {
    const sourceId = path.getAttribute('data-source-id');
    const receiverId = path.getAttribute('data-receiver-id');
    const receiverIdsStr = path.getAttribute('data-receiver-ids');
    const sourceIdsStr = path.getAttribute('data-source-ids');

    // Split and filter to avoid empty strings and 'null' string
    const receiverIds =
      receiverIdsStr && receiverIdsStr !== 'null'
        ? receiverIdsStr.split(',').filter((id) => id.trim())
        : [];
    const sourceIds =
      sourceIdsStr && sourceIdsStr !== 'null'
        ? sourceIdsStr.split(',').filter((id) => id.trim())
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

    // Apply opacity - hide unrelated lines when hovering
    if (hoveredSourceId || hoveredReceiverId) {
      path.style.opacity = shouldHighlight ? '1' : '0';
    } else {
      path.style.opacity = '1';
    }
  });
}
