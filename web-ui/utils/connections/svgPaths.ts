/**
 * Calculate SVG path from source account to relay server
 *
 * @param sourceRect - Bounding rectangle of the source element
 * @param svgRect - Bounding rectangle of the SVG container
 * @param centerX - X coordinate of the relay server center
 * @param centerY - Y coordinate of the relay server center
 * @param radius - Corner radius for rounded path
 * @returns SVG path data string
 */
export function calculateSourceToServerPath(
  sourceRect: DOMRect,
  svgRect: DOMRect,
  centerX: number,
  centerY: number,
  radius: number = 8
): string {
  const x1 = sourceRect.right - svgRect.left;
  const y1 = sourceRect.top - svgRect.top + sourceRect.height / 2;

  const verticalX = centerX - 30;

  // Path with rounded corners
  let pathData = `M ${x1} ${y1}`;

  if (Math.abs(y1 - centerY) > radius * 2) {
    // Horizontal line
    pathData += ` L ${verticalX - radius} ${y1}`;
    // Rounded corner
    if (y1 < centerY) {
      pathData += ` Q ${verticalX} ${y1}, ${verticalX} ${y1 + radius}`;
    } else {
      pathData += ` Q ${verticalX} ${y1}, ${verticalX} ${y1 - radius}`;
    }
    // Vertical line
    pathData += ` L ${verticalX} ${centerY - (y1 < centerY ? radius : -radius)}`;
    // Rounded corner to server
    pathData += ` Q ${verticalX} ${centerY}, ${verticalX + radius} ${centerY}`;
  } else {
    pathData += ` L ${verticalX} ${y1} L ${verticalX} ${centerY}`;
  }

  // Final horizontal to server
  pathData += ` L ${centerX - 20} ${centerY}`;

  return pathData;
}

/**
 * Calculate SVG path from relay server to receiver account
 *
 * @param receiverRect - Bounding rectangle of the receiver element
 * @param svgRect - Bounding rectangle of the SVG container
 * @param centerX - X coordinate of the relay server center
 * @param centerY - Y coordinate of the relay server center
 * @param radius - Corner radius for rounded path
 * @returns SVG path data string
 */
export function calculateServerToReceiverPath(
  receiverRect: DOMRect,
  svgRect: DOMRect,
  centerX: number,
  centerY: number,
  radius: number = 8
): string {
  const x2 = receiverRect.left - svgRect.left;
  const y2 = receiverRect.top - svgRect.top + receiverRect.height / 2;

  const verticalX = centerX + 30;

  // Path with rounded corners
  let pathData = `M ${centerX + 20} ${centerY}`;

  // Horizontal from server
  pathData += ` L ${verticalX - radius} ${centerY}`;

  if (Math.abs(y2 - centerY) > radius * 2) {
    // Rounded corner from horizontal to vertical
    if (y2 < centerY) {
      pathData += ` Q ${verticalX} ${centerY}, ${verticalX} ${centerY - radius}`;
    } else {
      pathData += ` Q ${verticalX} ${centerY}, ${verticalX} ${centerY + radius}`;
    }
    // Vertical line
    pathData += ` L ${verticalX} ${y2 - (y2 < centerY ? -radius : radius)}`;
    // Rounded corner to horizontal
    pathData += ` Q ${verticalX} ${y2}, ${verticalX + radius} ${y2}`;
  } else {
    pathData += ` L ${verticalX} ${centerY} L ${verticalX} ${y2}`;
  }

  // Final horizontal to receiver
  pathData += ` L ${x2} ${y2}`;

  return pathData;
}

/**
 * Calculate the center position for the relay server icon
 *
 * @param elements - Array of account elements to calculate center from
 * @param svgRect - Bounding rectangle of the SVG container
 * @returns Object with x and y coordinates
 */
export function calculateCenterPosition(
  elements: HTMLElement[],
  svgRect: DOMRect
): { x: number; y: number } {
  const width = svgRect.width;

  if (elements.length === 0) {
    return {
      x: width / 2,
      y: svgRect.height / 2,
    };
  }

  let totalY = 0;
  let count = 0;

  elements.forEach((el) => {
    if (el) {
      const rect = el.getBoundingClientRect();
      totalY += rect.top - svgRect.top + rect.height / 2;
      count++;
    }
  });

  return {
    x: width / 2,
    y: count > 0 ? totalY / count : svgRect.height / 2,
  };
}

/**
 * Create SVG relay server icon group element (for DOM manipulation approach)
 *
 * @param centerX - X coordinate of the center
 * @param centerY - Y coordinate of the center
 * @param svgns - SVG namespace URI
 * @returns SVG group element with server icon
 */
export function createRelayServerIcon(
  centerX: number,
  centerY: number,
  svgns: string
): SVGGElement {
  const serverGroup = document.createElementNS(svgns, 'g');
  serverGroup.setAttribute('transform', `translate(${centerX - 20}, ${centerY - 20})`);

  // Background circle
  const circle = document.createElementNS(svgns, 'circle');
  circle.setAttribute('cx', '20');
  circle.setAttribute('cy', '20');
  circle.setAttribute('r', '20');
  circle.setAttribute('fill', '#3b82f6');
  serverGroup.appendChild(circle);

  // Server icon (simplified rectangle representation)
  for (let i = 0; i < 3; i++) {
    const serverRect = document.createElementNS(svgns, 'rect');
    serverRect.setAttribute('x', '12');
    serverRect.setAttribute('y', String(12 + i * 6));
    serverRect.setAttribute('width', '16');
    serverRect.setAttribute('height', '4');
    serverRect.setAttribute('rx', '1');
    serverRect.setAttribute('fill', 'white');
    serverGroup.appendChild(serverRect);
  }

  return serverGroup;
}
