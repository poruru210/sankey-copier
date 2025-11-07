# React Flow Drag Functionality Fix

## Summary
Fixed the drag functionality for account nodes in React Flow by implementing proper event handling patterns.

## Changes Made

### 1. AccountCardHeader.tsx (web-ui/components/connections/AccountCardHeader.tsx:34-49)
```typescript
// Added cursor-move to show draggable area
<div className={`flex items-center gap-1 md:gap-2 px-2 md:px-3 py-2 cursor-move ...`}>

  // Added pointer-events-none to icon (prevents event capture)
  <div className="... pointer-events-none">
    <Folder ... />
  </div>

  // Added pointer-events-none to text area (allows drag to work)
  <div className="flex-1 min-w-0 pointer-events-none">
    <h3 ...>{account.name}</h3>
  </div>

  // Kept noDrag on interactive elements
  <div className="noDrag">
    <Switch ... />
  </div>
  <button className="noDrag ...">
    <Settings />
  </button>
  <button className="noDrag ...">
    <ChevronDown />
  </button>
</div>
```

### 2. AccountCard.tsx (web-ui/components/connections/AccountCard.tsx:82-88)
```typescript
// Removed touch-manipulation class that was interfering with drag
<div
  className={`bg-white dark:bg-gray-800 rounded-lg overflow-hidden shadow-lg ${
    isMobile ? 'flex flex-col' : 'flex'
  } transition-all w-full text-sm md:text-base ${getVisibilityClass()}`}
>
```

### 3. AccountCardExpanded.tsx (web-ui/components/connections/AccountCardExpanded.tsx:41-43)
```typescript
// Added cursor-move and pointer-events-none to expanded content
<div className="border-t border-gray-200 dark:border-gray-700 cursor-move">
  <div className="px-2 md:px-3 py-2 md:py-3 bg-gray-50 dark:bg-gray-900/30">
    <div className="space-y-2 md:space-y-3 pointer-events-none">
```

## Technical Explanation

### How React Flow Drag Works:
1. React Flow detects `mousedown` events on nodes to initiate drag
2. Elements with `.noDrag` class prevent drag from starting
3. All other areas should allow drag to initiate

### The Problem:
- Text and icon elements were capturing mouse events
- The `touch-manipulation` CSS property was interfering
- Mouse events weren't reaching React Flow's drag handlers

### The Solution:
1. **`pointer-events-none`**: Applied to non-interactive content (icons, text)
   - This makes mouse events "pass through" these elements
   - Events bubble up to the parent div which can initiate drag

2. **`cursor-move`**: Applied to draggable areas
   - Provides visual feedback that area is draggable
   - Shows when hovering over headers and expanded content

3. **`noDrag` class**: Kept on interactive elements
   - Prevents drag when clicking buttons or toggles
   - Ensures UI controls work correctly

4. **Removed `touch-manipulation`**:
   - This CSS property was interfering with drag behavior
   - Not necessary for our use case

## Manual Verification Steps

### Pre-requisites:
1. Server running: `cd web-ui && pnpm dev`
2. Open browser: http://localhost:5173

### Test 1: Visual Feedback
✓ Hover over account card header → cursor should change to move cursor
✓ Hover over expanded content → cursor should change to move cursor
✓ Hover over buttons/switches → cursor remains pointer

### Test 2: Drag Account Nodes
1. Click and hold on the header area (where account name is)
2. Drag mouse to reposition the node
3. Release mouse
4. ✓ Node should move to new position
5. ✓ RelayServer node should remain centered

### Test 3: Interactive Elements Still Work
1. Click the Switch toggle → ✓ Should toggle on/off
2. Click the Settings button → ✓ Should expand settings
3. Click the expand/collapse button → ✓ Should expand card details
4. Click delete button in settings → ✓ Should delete setting
5. None of these clicks should initiate drag

### Test 4: Hover Highlighting
1. Hover over a source account
2. ✓ Connected edges and receiver accounts should highlight
3. ✓ Drag should still work while highlighted

## Architecture Notes

### Event Flow:
```
User clicks on account name/icon
  ↓
pointer-events-none → event passes through
  ↓
Event reaches parent div with cursor-move
  ↓
React Flow's drag handler captures mousedown
  ↓
Drag initiated ✓
```

### Interactive Elements:
```
User clicks on button with noDrag
  ↓
React Flow sees noDrag class
  ↓
Drag NOT initiated
  ↓
Button's onClick handler fires ✓
```

## Automated Testing

A test script has been created at `web-ui/test_drag_manual.py` but cannot run due to environment limitations (X server requirements for headless browser).

To run tests in your environment:
```bash
cd web-ui
pip install playwright
python -m playwright install chromium
python test_drag_manual.py
```

## Files Changed:
- web-ui/components/connections/AccountCardHeader.tsx
- web-ui/components/connections/AccountCard.tsx
- web-ui/components/connections/AccountCardExpanded.tsx
- web-ui/test_drag_manual.py (test script)

## Commit:
```
commit 53d4db6
Fix React Flow drag functionality with proper event handling
```

## Next Steps:
Please verify the drag functionality works correctly in your browser. If there are still issues, please provide specific feedback:
- What happens when you try to drag?
- Do the buttons/switches still work?
- Does the cursor change when hovering?
- Any console errors in browser dev tools?
