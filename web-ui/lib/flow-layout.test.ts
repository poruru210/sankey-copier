import { describe, it, expect } from 'vitest';
import { calculateInitialPosition } from './flow-layout';
import {
  VERTICAL_SPACING,
  SOURCE_X,
  RECEIVER_X,
  MOBILE_X,
  MOBILE_SOURCE_START_Y,
  MOBILE_VERTICAL_SPACING,
  MOBILE_SECTION_GAP,
} from '@/constants/flow-layout';

describe('calculateInitialPosition', () => {
  describe('Desktop Layout', () => {
    const isMobile = false;
    const sourceCount = 5; // Arbitrary number, not used for desktop

    it('should calculate correct positions for source nodes', () => {
      const index = 0;
      const pos = calculateInitialPosition(index, 'source', isMobile, sourceCount);
      expect(pos).toEqual({ x: SOURCE_X, y: 0 });

      const index2 = 2;
      const pos2 = calculateInitialPosition(index2, 'source', isMobile, sourceCount);
      expect(pos2).toEqual({ x: SOURCE_X, y: index2 * VERTICAL_SPACING });
    });

    it('should calculate correct positions for receiver nodes', () => {
      const index = 0;
      const pos = calculateInitialPosition(index, 'receiver', isMobile, sourceCount);
      expect(pos).toEqual({ x: RECEIVER_X, y: 0 });

      const index2 = 3;
      const pos2 = calculateInitialPosition(index2, 'receiver', isMobile, sourceCount);
      expect(pos2).toEqual({ x: RECEIVER_X, y: index2 * VERTICAL_SPACING });
    });
  });

  describe('Mobile Layout', () => {
    const isMobile = true;
    const sourceCount = 3;

    it('should calculate correct positions for source nodes', () => {
      const index = 0;
      const pos = calculateInitialPosition(index, 'source', isMobile, sourceCount);
      expect(pos).toEqual({ x: MOBILE_X, y: MOBILE_SOURCE_START_Y });

      const index2 = 1;
      const pos2 = calculateInitialPosition(index2, 'source', isMobile, sourceCount);
      expect(pos2).toEqual({
        x: MOBILE_X,
        y: MOBILE_SOURCE_START_Y + index2 * MOBILE_VERTICAL_SPACING,
      });
    });

    it('should calculate correct positions for receiver nodes', () => {
      // Receivers start after all sources + gap
      const index = 0;
      const pos = calculateInitialPosition(index, 'receiver', isMobile, sourceCount);
      const expectedY =
        MOBILE_SOURCE_START_Y +
        sourceCount * MOBILE_VERTICAL_SPACING +
        MOBILE_SECTION_GAP;
      expect(pos).toEqual({ x: MOBILE_X, y: expectedY });

      const index2 = 2;
      const pos2 = calculateInitialPosition(index2, 'receiver', isMobile, sourceCount);
      expect(pos2).toEqual({
        x: MOBILE_X,
        y: expectedY + index2 * MOBILE_VERTICAL_SPACING,
      });
    });
  });
});
