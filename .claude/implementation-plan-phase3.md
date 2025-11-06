# Implementation Plan: Phase 3 - Sidebar Filter UX
## Master Account Based Navigation System

**Created**: 2025-11-06
**Design Reference**: `.claude/design/sidebar-filter-ux.md`
**Priority**: High
**Estimated Duration**: 4-6 days

---

## Overview

This phase implements a sidebar-based filtering system that allows users to focus on specific master accounts and their connections, improving usability and scalability.

**Key Goals**:
1. Add master account sidebar navigation
2. Implement connection filtering by master
3. Enhance settings management UI
4. Maintain mobile responsiveness
5. Preserve existing functionality

---

## Phase Breakdown

### Phase 3.1: Foundation & Sidebar UI
**Duration**: 1-2 days
**Focus**: Create sidebar component structure

#### Tasks

1. **Create Sidebar Component Structure**
   - Create `web-ui/components/sidebar/MasterAccountSidebar.tsx`
   - Create `web-ui/components/sidebar/MasterAccountItem.tsx`
   - Add TypeScript interfaces for sidebar state
   - **Files to create**:
     - `web-ui/components/sidebar/MasterAccountSidebar.tsx`
     - `web-ui/components/sidebar/MasterAccountItem.tsx`
   - **Estimated**: 2-3 hours

2. **Update ConnectionsView Layout**
   - Modify `web-ui/components/ConnectionsView.tsx` to include sidebar
   - Add CSS Grid layout for desktop (sidebar + main)
   - Add state for `selectedMasterId`
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 2 hours

3. **Implement Sidebar Visual Design**
   - Style sidebar with Tailwind CSS
   - Add "All Accounts" option
   - Display master account list with status indicators
   - Show connection count per master
   - **Files to modify**:
     - `web-ui/components/sidebar/MasterAccountSidebar.tsx`
     - `web-ui/components/sidebar/MasterAccountItem.tsx`
   - **Estimated**: 3-4 hours

4. **Add Mobile Drawer Functionality**
   - Implement drawer/overlay for mobile
   - Add hamburger menu button
   - Handle backdrop click to close
   - **Files to modify**:
     - `web-ui/components/sidebar/MasterAccountSidebar.tsx`
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 2-3 hours

**Deliverables**:
- Functional sidebar displaying all master accounts
- Responsive layout (desktop + mobile)
- No filtering logic yet (all accounts visible)

**Testing**:
- Visual regression test (desktop/tablet/mobile)
- Sidebar opens/closes on mobile
- All master accounts appear in sidebar

---

### Phase 3.2: Filter Logic Implementation
**Duration**: 1 day
**Focus**: Implement filtering behavior

#### Tasks

1. **Implement Filter State Management**
   - Add `selectedMasterId` state to ConnectionsView
   - Create handler for master selection
   - Persist selection in component state (not localStorage yet)
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 1 hour

2. **Create Filtering Logic**
   - Implement `visibleMasters` computed value
   - Implement `visibleSlaves` computed value
   - Implement `visibleConnections` computed value
   - Use `useMemo` for performance
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 2 hours

3. **Update Connection Rendering**
   - Pass filtered data to child components
   - Update connection lines to only show visible pairs
   - Add filter indicator at top of main view
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
     - `web-ui/components/connections/ConnectionLines.tsx`
   - **Estimated**: 2-3 hours

4. **Add Clear Filter Functionality**
   - Add "× Clear" button in filter indicator
   - Reset to "All Accounts" on click
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 1 hour

**Deliverables**:
- Clicking a master in sidebar filters the main view
- Only selected master and its slaves visible
- Connection lines update correctly
- "All Accounts" shows everything (default behavior)

**Testing**:
- Filter by specific master → Only that master's connections visible
- Switch between different masters → Correct slaves shown
- Click "All Accounts" → All connections visible
- Connection lines render only for visible pairs

---

### Phase 3.3: Enhanced Settings Management
**Duration**: 1-2 days
**Focus**: Improve settings UI with inline details

#### Tasks

1. **Enhance AccountCardHeader Settings Display**
   - Show more details per setting (lot multiplier, reverse trade)
   - Improve visual hierarchy
   - Add hover states
   - **Files to modify**:
     - `web-ui/components/connections/AccountCardHeader.tsx`
   - **Estimated**: 2 hours

2. **Create Detailed Settings Modal**
   - Create `web-ui/components/dialogs/DetailedSettingsModal.tsx`
   - Include all CopySettings fields:
     - Basic: lot_multiplier, reverse_trade, enabled
     - Filters: allowed_symbols, blocked_symbols, magic_numbers
     - Symbol mappings array
   - Use tabs or sections for organization
   - **Files to create**:
     - `web-ui/components/dialogs/DetailedSettingsModal.tsx`
   - **Estimated**: 4-5 hours

3. **Integrate Modal with Settings List**
   - Make each setting item clickable
   - Open modal on click with selected setting data
   - Handle save → PUT request to backend
   - Update local state on success
   - **Files to modify**:
     - `web-ui/components/connections/AccountCardHeader.tsx`
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 2-3 hours

4. **Implement Delete Confirmation**
   - Replace `window.confirm` with custom confirmation dialog
   - Show connection details in confirmation
   - Handle delete → DELETE request to backend
   - Update local state on success
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Files to create**:
     - `web-ui/components/dialogs/DeleteConfirmationDialog.tsx`
   - **Estimated**: 2 hours

**Deliverables**:
- Gear icon → Settings list with details
- Click setting → Opens detailed modal
- Edit all parameters (basic + advanced)
- Delete with confirmation dialog
- Toast notifications on success/error

**Testing**:
- Open settings list → Details displayed correctly
- Click setting → Modal opens with correct data
- Edit and save → Changes persisted
- Delete setting → Confirmation dialog → Setting deleted
- Error handling → Toast notifications

---

### Phase 3.4: Animations & Polish
**Duration**: 0.5-1 day
**Focus**: Smooth transitions and visual feedback

#### Tasks

1. **Add Filter Change Animations**
   - Fade out hidden cards
   - Slide remaining cards to position
   - Fade in filter indicator
   - Use Framer Motion or CSS transitions
   - **Files to modify**:
     - `web-ui/components/ConnectionsView.tsx`
   - **Estimated**: 2-3 hours

2. **Improve Sidebar Interactions**
   - Hover states on master items
   - Active state styling
   - Smooth collapse/expand animation (desktop)
   - Drawer slide-in animation (mobile)
   - **Files to modify**:
     - `web-ui/components/sidebar/MasterAccountSidebar.tsx`
   - **Estimated**: 2 hours

3. **Settings List Animation**
   - Smooth expand/collapse (already partially done)
   - Fade in setting items
   - Stagger animation for multiple items
   - **Files to modify**:
     - `web-ui/components/connections/AccountCardHeader.tsx`
   - **Estimated**: 1-2 hours

**Deliverables**:
- Smooth filter transitions
- Polished sidebar interactions
- Animated settings expansions

**Testing**:
- Visual QA on all animations
- Performance testing (no janky animations)
- Test on different devices/browsers

---

### Phase 3.5: i18n, Accessibility & Testing
**Duration**: 1 day
**Focus**: Internationalization, a11y, comprehensive testing

#### Tasks

1. **Add i18n Strings**
   - Add English translations
   - Add Japanese translations
   - Strings needed:
     - "All Accounts", "Filter Accounts", "Viewing: {name}"
     - "Copy Settings", "Edit Settings", "Delete Setting"
     - "No connections configured", "Create Link"
     - Confirmation messages
   - **Files to modify**:
     - `web-ui/intlayer/en/connections.content.ts`
     - `web-ui/intlayer/ja/connections.content.ts`
   - **Estimated**: 1-2 hours

2. **Implement Accessibility**
   - Add ARIA labels to sidebar items
   - Keyboard navigation (tab, arrow keys, escape)
   - Screen reader announcements on filter change
   - Focus management (modal open/close)
   - **Files to modify**:
     - `web-ui/components/sidebar/MasterAccountSidebar.tsx`
     - `web-ui/components/dialogs/DetailedSettingsModal.tsx`
   - **Estimated**: 2-3 hours

3. **Write Playwright Tests**
   - Test sidebar filtering workflow
   - Test settings CRUD operations
   - Test mobile drawer behavior
   - Test keyboard navigation
   - **Files to create**:
     - `test_sidebar_filter.py`
     - `test_settings_management.py`
   - **Estimated**: 3-4 hours

4. **Manual QA**
   - Test on different screen sizes
   - Test with varying data (1 master, 10 masters, 50 masters)
   - Test edge cases (no connections, orphaned slaves)
   - Test error scenarios (API failures)
   - **Estimated**: 2 hours

**Deliverables**:
- Full i18n support (EN + JA)
- WCAG 2.1 AA compliance
- Comprehensive test coverage
- QA sign-off

**Testing**:
- Run all Playwright tests → Pass
- Manual accessibility audit → Pass
- Test in both languages → Translations correct

---

## Technical Specifications

### New Components

#### `MasterAccountSidebar.tsx`
```typescript
interface MasterAccountSidebarProps {
  masters: MasterAccountInfo[];
  selectedMasterId: string | 'all';
  onSelectMaster: (id: string | 'all') => void;
  isMobile: boolean;
  isOpen?: boolean;  // For mobile drawer
  onClose?: () => void;  // For mobile drawer
  content: SidebarContent;  // i18n
}

interface MasterAccountInfo {
  id: string;
  name: string;
  status: 'online' | 'offline' | 'error';
  connectionCount: number;
  hasActiveErrors: boolean;
}
```

#### `DetailedSettingsModal.tsx`
```typescript
interface DetailedSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  setting: CopySettings;
  onSave: (updated: CopySettings) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  content: SettingsModalContent;  // i18n
}
```

### State Management

#### ConnectionsView State
```typescript
interface ConnectionsViewState {
  // Existing
  connections: EaConnection[];
  copySettings: CopySettings[];
  dialogOpen: boolean;
  editingSettings: CopySettings | null;

  // New
  selectedMasterId: string | 'all';  // Filter state
  sidebarOpen: boolean;  // Mobile drawer state
  detailedSettingsModalOpen: boolean;
  selectedSettingForDetail: CopySettings | null;
  deleteConfirmationOpen: boolean;
  settingToDelete: CopySettings | null;
}
```

### API Endpoints

No new endpoints required. Existing endpoints:
- `GET /api/settings` - Fetch all copy settings
- `POST /api/settings` - Create new setting
- `PUT /api/settings/:id` - Update setting
- `DELETE /api/settings/:id` - Delete setting

DetailedSettingsModal will use PUT to update all fields.

---

## Data Flow Diagram

```
┌──────────────────────────────────────────────────────────┐
│  ConnectionsView (Parent State)                          │
│                                                           │
│  State:                                                   │
│  - connections: EaConnection[]                            │
│  - copySettings: CopySettings[]                           │
│  - selectedMasterId: string | 'all'                       │
│                                                           │
│  Computed (useMemo):                                      │
│  - masterAccounts: MasterAccountInfo[]                    │
│  - visibleMasters: EaConnection[]                         │
│  - visibleSlaves: EaConnection[]                          │
│  - visibleConnections: CopySettings[]                     │
└────────────┬─────────────────────────────────────────────┘
             │
             ├─────────────────────────┬─────────────────────
             ▼                         ▼
┌──────────────────────┐   ┌──────────────────────────────┐
│ MasterAccountSidebar │   │ Filtered Connection View     │
│                      │   │                              │
│ Props:               │   │ Props:                       │
│ - masterAccounts     │   │ - masters: visibleMasters    │
│ - selectedMasterId   │   │ - slaves: visibleSlaves      │
│ - onSelectMaster     │   │ - connections:               │
│                      │   │   visibleConnections         │
│ Renders:             │   │                              │
│ - "All Accounts"     │   │ Renders:                     │
│ - Master list        │   │ - Master cards (filtered)    │
│ - Connection counts  │   │ - Slave cards (filtered)     │
│                      │   │ - Connection lines           │
│ Events:              │   │                              │
│ onClick ──────────►  │   │ Settings gear icon:          │
│ update filter        │   │ - Show settings list         │
└──────────────────────┘   │ - Click → DetailedModal      │
                           │ - Delete → Confirmation      │
                           └──────────────────────────────┘
```

---

## Risk Assessment & Mitigation

### Risk 1: Performance Degradation with Many Masters
**Likelihood**: Medium
**Impact**: High
**Mitigation**:
- Use `React.memo` on all cards
- Use `useMemo` for filtering logic
- Implement virtualization if > 50 masters (react-window)
- Lazy load connection lines (render on viewport)

### Risk 2: Breaking Existing Functionality
**Likelihood**: Medium
**Impact**: High
**Mitigation**:
- Thorough testing after each phase
- Keep "All Accounts" as default (preserves current UX)
- Feature flag to enable/disable sidebar (if needed)
- Comprehensive Playwright test coverage

### Risk 3: Mobile UX Complexity
**Likelihood**: Low
**Impact**: Medium
**Mitigation**:
- Early testing on real mobile devices
- Use established drawer/overlay patterns
- Keep mobile controls simple and touch-friendly

### Risk 4: i18n Strings Incomplete
**Likelihood**: Low
**Impact**: Low
**Mitigation**:
- Create comprehensive string list upfront
- Review with native Japanese speaker
- Use fallback to English for missing translations

---

## Dependencies

### External Libraries
No new dependencies required. Using existing:
- React 19.0.0
- TypeScript
- Tailwind CSS
- Lucide icons
- Intlayer (i18n)

### Optional (for animations)
- `framer-motion` - If CSS transitions insufficient
  - Install: `pnpm add framer-motion`
  - Use sparingly to avoid bundle bloat

---

## Testing Strategy

### Unit Tests (Jest/Vitest)
- Filter logic: `visibleMasters`, `visibleSlaves`, `visibleConnections`
- Master account info derivation
- Edge cases: Empty data, single master, no connections

### Integration Tests (Playwright)
1. **Sidebar Navigation**
   - Click "All Accounts" → All visible
   - Click specific master → Only that master's connections visible
   - Switch between masters → Correct filtering

2. **Settings Management**
   - Open settings list → Items displayed
   - Click setting → Modal opens
   - Edit → Save → Changes persisted
   - Delete → Confirmation → Setting removed

3. **Mobile Behavior**
   - Hamburger menu → Sidebar opens
   - Backdrop click → Sidebar closes
   - Filter selection → Sidebar auto-closes

4. **Keyboard Navigation**
   - Tab through sidebar
   - Arrow keys navigate masters
   - Escape clears filter / closes modals

### Manual QA Checklist
- [ ] Sidebar displays all masters with correct counts
- [ ] Filter works correctly for each master
- [ ] "All Accounts" shows everything
- [ ] Settings list displays correct connections
- [ ] Detailed modal shows all fields
- [ ] Edit → Save → Updates correctly
- [ ] Delete → Confirmation → Removes correctly
- [ ] Mobile drawer opens/closes smoothly
- [ ] Animations are smooth (60fps)
- [ ] No console errors or warnings
- [ ] Works in Chrome, Firefox, Safari, Edge
- [ ] Works on iOS and Android
- [ ] English translations correct
- [ ] Japanese translations correct
- [ ] Keyboard navigation works
- [ ] Screen reader announces correctly

---

## Success Criteria

### Functional
- ✅ Sidebar displays all master accounts with status and count
- ✅ Clicking a master filters the main view
- ✅ "All Accounts" shows all connections (default)
- ✅ Settings list shows connection details
- ✅ Detailed modal allows editing all CopySettings fields
- ✅ Delete with confirmation works correctly
- ✅ Mobile drawer functions properly

### Non-Functional
- ✅ Filter change < 200ms (smooth performance)
- ✅ No layout shift or jank
- ✅ Responsive on all screen sizes (320px - 4K)
- ✅ WCAG 2.1 AA compliance
- ✅ Full i18n support (EN + JA)
- ✅ Test coverage > 80%

---

## Rollout Plan

### Phase 1: Internal Testing
- Deploy to dev environment
- Internal team testing
- Collect feedback
- Fix critical issues

### Phase 2: Beta Release
- Deploy to staging
- Invite select users for beta testing
- Monitor usage metrics
- Gather user feedback

### Phase 3: Production Release
- Deploy to production
- Monitor error logs and performance
- Be ready for quick hotfixes
- Collect user satisfaction data

### Phase 4: Iteration
- Analyze usage patterns
- Identify pain points
- Plan next improvements (search, favorites, etc.)

---

## Post-Launch Monitoring

### Metrics to Track
1. **Adoption**: % of users who use filtering vs "All Accounts"
2. **Efficiency**: Time to complete "edit specific setting" task
3. **Errors**: Error rate when editing/deleting settings
4. **Performance**: Page load time, filter change time
5. **Satisfaction**: User feedback scores

### Alerts to Configure
- API error rate spikes
- Slow filter operations (> 500ms)
- High rate of modal abandonment (open without save/cancel)

---

## File Structure

After implementation, new files:

```
web-ui/
├── components/
│   ├── sidebar/
│   │   ├── MasterAccountSidebar.tsx      [NEW]
│   │   └── MasterAccountItem.tsx         [NEW]
│   ├── dialogs/
│   │   ├── DetailedSettingsModal.tsx     [NEW]
│   │   └── DeleteConfirmationDialog.tsx  [NEW]
│   ├── ConnectionsView.tsx               [MODIFIED]
│   └── connections/
│       ├── AccountCardHeader.tsx         [MODIFIED]
│       └── ConnectionLines.tsx           [MODIFIED - optional]
├── intlayer/
│   ├── en/
│   │   └── connections.content.ts        [MODIFIED]
│   └── ja/
│       └── connections.content.ts        [MODIFIED]

.claude/
├── design/
│   └── sidebar-filter-ux.md              [NEW - Created]
├── implementation-plan-phase3.md         [NEW - This file]

tests/ (or root)
├── test_sidebar_filter.py                [NEW]
└── test_settings_management.py           [NEW]
```

---

## Timeline Summary

| Phase | Duration | Key Deliverable |
|-------|----------|-----------------|
| 3.1: Foundation & Sidebar UI | 1-2 days | Functional sidebar (no filtering) |
| 3.2: Filter Logic | 1 day | Working filter by master |
| 3.3: Enhanced Settings | 1-2 days | Detailed modal, delete confirmation |
| 3.4: Animations & Polish | 0.5-1 day | Smooth transitions |
| 3.5: i18n, A11y, Testing | 1 day | Full i18n, tests, QA |
| **Total** | **4-6 days** | **Complete sidebar filter UX** |

---

## Next Steps

1. **Review this plan** with stakeholders
2. **Answer design questions** from sidebar-filter-ux.md Section 13
3. **Set up dev environment** (ensure all dependencies installed)
4. **Create feature branch**: `git checkout -b feature/sidebar-filter-ux`
5. **Begin Phase 3.1**: Create MasterAccountSidebar component

---

## Questions for Approval

Before starting implementation:

1. **Persistence**: Should we persist selected filter in localStorage?
   - Pro: User preference maintained across sessions
   - Con: May confuse users if they forget filter is active

2. **Pre-selection in Create Dialog**: When a master is selected in sidebar, should "Create New Link" pre-select that master?
   - Pro: Faster workflow
   - Con: User may not expect pre-selection

3. **Advanced Filters UI**: How detailed should symbol/magic number filtering be in the modal?
   - Option A: Simple text input (comma-separated)
   - Option B: Dynamic list with add/remove buttons
   - Recommendation: Start with A, enhance to B later

4. **Connection Line Rendering**: For complex graphs (many connections), should we:
   - Option A: Use SVG (current)
   - Option B: Switch to Canvas for > 20 connections
   - Recommendation: Keep SVG for now, optimize if performance issues arise

---

**End of Implementation Plan**

**Ready to begin?** Confirm approval and we'll start with Phase 3.1.
