import { useEffect } from 'react';
import type { AccountInfo } from '@/types';
import {
  calculateCenterPosition,
  createSourceToServerPath,
  createSourceToServerPathMobile,
  createServerToReceiverPath,
  createServerToReceiverPathMobile,
  createPathElement,
  createRelayServerIcon,
  updateLineOpacity,
  isMobileViewport,
} from '@/utils/connections/svgDrawing';

interface UseSVGConnectionsProps {
  sourceAccounts: AccountInfo[];
  receiverAccounts: AccountInfo[];
  sourceRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>;
  receiverRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>;
  middleColumnRef: React.RefObject<HTMLDivElement | null>;
  hoveredSourceId: string | null;
  hoveredReceiverId: string | null;
  getConnectedReceivers: (sourceId: string) => string[];
  getConnectedSources: (receiverId: string) => string[];
}

/**
 * Custom hook to manage SVG connection line drawing and updates
 */
export function useSVGConnections({
  sourceAccounts,
  receiverAccounts,
  sourceRefs,
  receiverRefs,
  middleColumnRef,
  hoveredSourceId,
  hoveredReceiverId,
  getConnectedReceivers,
  getConnectedSources,
}: UseSVGConnectionsProps) {
  useEffect(() => {
    const drawLines = () => {
      const svg = document.getElementById('connection-svg') as unknown as SVGSVGElement;
      if (!svg) return;

      // Remove existing elements
      svg.querySelectorAll('path, g').forEach((el) => el.remove());

      const svgRect = svg.getBoundingClientRect();
      const isMobile = isMobileViewport();

      // Calculate center position for the relay server
      const { centerX, centerY } = calculateCenterPosition(
        sourceRefs,
        receiverRefs,
        sourceAccounts,
        receiverAccounts,
        svgRect,
        middleColumnRef
      );

      // Draw lines from sources to center
      sourceAccounts.forEach((source) => {
        const sourceEl = sourceRefs.current[source.id];
        if (!sourceEl) return;

        const sourceRect = sourceEl.getBoundingClientRect();

        // Mobile: use center X, bottom Y; Desktop: use right X, center Y
        const x1 = isMobile
          ? sourceRect.left - svgRect.left + sourceRect.width / 2
          : sourceRect.right - svgRect.left;
        const y1 = isMobile
          ? sourceRect.bottom - svgRect.top
          : sourceRect.top - svgRect.top + sourceRect.height / 2;

        // Use mobile or desktop path generation
        const pathData = isMobile
          ? createSourceToServerPathMobile(x1, y1, centerX, centerY)
          : createSourceToServerPath(x1, y1, centerX, centerY);

        const isActive = source.isEnabled && !source.hasError;
        const connectedReceivers = getConnectedReceivers(source.id);

        const path = createPathElement(pathData, isActive, {
          'data-source-id': source.id,
          'data-receiver-ids': connectedReceivers.join(','),
        });

        svg.appendChild(path);
      });

      // Draw lines from center to receivers
      receiverAccounts.forEach((receiver) => {
        const receiverEl = receiverRefs.current[receiver.id];
        if (!receiverEl) return;

        const receiverRect = receiverEl.getBoundingClientRect();

        // Mobile: use center X, top Y; Desktop: use left X, center Y
        const x2 = isMobile
          ? receiverRect.left - svgRect.left + receiverRect.width / 2
          : receiverRect.left - svgRect.left;
        const y2 = isMobile
          ? receiverRect.top - svgRect.top
          : receiverRect.top - svgRect.top + receiverRect.height / 2;

        // Use mobile or desktop path generation
        const pathData = isMobile
          ? createServerToReceiverPathMobile(x2, y2, centerX, centerY)
          : createServerToReceiverPath(x2, y2, centerX, centerY);

        const isActive = receiver.isEnabled && !receiver.hasError && !receiver.hasWarning;
        const connectedSources = getConnectedSources(receiver.id);

        const path = createPathElement(pathData, isActive, {
          'data-receiver-id': receiver.id,
          'data-source-ids': connectedSources.join(','),
        });

        svg.appendChild(path);
      });

      // Draw relay server icon at center
      const serverIcon = createRelayServerIcon(centerX, centerY, isMobile);
      svg.appendChild(serverIcon);

      // Apply hover state after drawing lines
      updateLineOpacity(svg, hoveredSourceId, hoveredReceiverId);
    };

    drawLines();

    // Setup ResizeObserver to redraw on element resize
    const resizeObserver = new ResizeObserver(drawLines);
    Object.values(sourceRefs.current).forEach((el) => {
      if (el) resizeObserver.observe(el);
    });
    Object.values(receiverRefs.current).forEach((el) => {
      if (el) resizeObserver.observe(el);
    });

    // Add window resize listener with debounce for better performance
    let resizeTimeout: NodeJS.Timeout;
    const handleWindowResize = () => {
      clearTimeout(resizeTimeout);
      resizeTimeout = setTimeout(drawLines, 100);
    };

    window.addEventListener('resize', handleWindowResize);

    // Also observe the SVG container and middle column for size changes
    const svg = document.getElementById('connection-svg') as unknown as SVGSVGElement;
    if (svg) {
      resizeObserver.observe(svg);
    }
    if (middleColumnRef.current) {
      resizeObserver.observe(middleColumnRef.current);
    }

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener('resize', handleWindowResize);
      clearTimeout(resizeTimeout);
    };
  }, [
    sourceAccounts,
    receiverAccounts,
    hoveredSourceId,
    hoveredReceiverId,
    getConnectedReceivers,
    getConnectedSources,
    sourceRefs,
    receiverRefs,
  ]);
}
