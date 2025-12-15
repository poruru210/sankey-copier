import {
  NODE_WIDTH,
  NODE_HEIGHT,
  VERTICAL_SPACING,
  SOURCE_X,
  RECEIVER_X,
  MOBILE_X,
  MOBILE_SOURCE_START_Y,
  MOBILE_VERTICAL_SPACING,
  MOBILE_SECTION_GAP,
} from '@/constants/flow-layout';

export interface XYPosition {
  x: number;
  y: number;
}

/**
 * Calculates the initial position of a node based on its index and type.
 * This is a pure function that does not depend on React state or DOM.
 *
 * @param index The index of the account in the list
 * @param type 'source' or 'receiver'
 * @param isMobile Whether the layout is for mobile
 * @param sourceCount Number of source accounts (needed for mobile receiver positioning)
 * @returns The calculated {x, y} position
 */
export function calculateInitialPosition(
  index: number,
  type: 'source' | 'receiver',
  isMobile: boolean,
  sourceCount: number = 0
): XYPosition {
  if (isMobile) {
    if (type === 'source') {
      return {
        x: MOBILE_X,
        y: MOBILE_SOURCE_START_Y + index * MOBILE_VERTICAL_SPACING,
      };
    } else {
      return {
        x: MOBILE_X,
        y:
          MOBILE_SOURCE_START_Y +
          sourceCount * MOBILE_VERTICAL_SPACING +
          MOBILE_SECTION_GAP +
          index * MOBILE_VERTICAL_SPACING,
      };
    }
  } else {
    // Desktop layout
    if (type === 'source') {
      return {
        x: SOURCE_X,
        y: index * VERTICAL_SPACING,
      };
    } else {
      return {
        x: RECEIVER_X,
        y: index * VERTICAL_SPACING,
      };
    }
  }
}
