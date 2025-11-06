# Sidebar + Filter UX Design
## Master Account Based Navigation System

**Created**: 2025-11-06
**Status**: Design Phase
**Priority**: High - Core UX Improvement

---

## 1. Design Philosophy

### Core Principle
**"Focus on the source, manage the flow"**

Users think in terms of:
- "Which accounts am I copying FROM?"
- "Where is this master account copying TO?"
- "What are the settings for this specific master?"

This design aligns the UI with the user's mental model by making master accounts the primary navigation unit.

---

## 2. Visual Layout

### 2.1 Overall Structure

```
┌─────────────────────────────────────────────────────────────┐
│  Header (unchanged)                                         │
├─────────────┬───────────────────────────────────────────────┤
│             │                                               │
│  Sidebar    │  Main Connection View                         │
│  (240px)    │                                               │
│             │  ┌──────────────┐     ┌──────────────┐        │
│  [Filter]   │  │ Master Card  │────▶│ Slave Card 1 │        │
│             │  └──────────────┘     └──────────────┘        │
│  Masters:   │                                               │
│  ○ All      │                       ┌──────────────┐        │
│  ● Acct A   │                  ────▶│ Slave Card 2 │        │
│  ○ Acct B   │                       └──────────────┘        │
│  ○ Acct C   │                                               │
│             │                                               │
│  [+ New]    │                                               │
│             │                                               │
└─────────────┴───────────────────────────────────────────────┘
```

### 2.2 Responsive Behavior

**Desktop (≥1024px)**:
- Sidebar: 240px fixed width
- Main view: Flexible width
- Both visible simultaneously

**Tablet (768px - 1023px)**:
- Sidebar: 200px fixed width
- Main view: Compressed layout
- Master cards stack vertically

**Mobile (<768px)**:
- Sidebar: Hidden by default, opens as overlay/drawer
- Hamburger menu icon to toggle
- Main view: Full width, vertical stack
- Filter shows as dropdown at top

---

## 3. Component Breakdown

### 3.1 Sidebar Component (`MasterAccountSidebar.tsx`)

**Purpose**: Master account navigation and filtering

**Features**:
- List all master accounts
- Show connection count per master
- "All Accounts" option
- Status indicators (online/offline/error)
- Collapsible for more space

**State**:
```typescript
interface SidebarState {
  selectedMasterId: string | 'all';
  masterAccounts: MasterAccountInfo[];
  isCollapsed: boolean;
}

interface MasterAccountInfo {
  id: string;
  name: string;
  status: 'online' | 'offline' | 'error';
  connectionCount: number;
  hasActiveErrors: boolean;
}
```

**Visual Design**:
```
┌─────────────────────┐
│ [≡] Filter Accounts │  ← Collapse button
├─────────────────────┤
│ 🔍 Search...        │  ← Quick search
├─────────────────────┤
│ ○ All Accounts (12) │  ← Default view
├─────────────────────┤
│ ● 📁 Account A      │  ← Selected
│   Online • 4 links  │
├─────────────────────┤
│ ○ 📁 Account B      │
│   Online • 2 links  │
├─────────────────────┤
│ ○ ⚠️ Account C      │  ← Has error
│   Offline • 3 links │
├─────────────────────┤
│                     │
│ [+ Create Link]     │  ← Quick action
└─────────────────────┘
```

### 3.2 Filtered Connection View (`FilteredConnectionsView.tsx`)

**Purpose**: Display connections based on selected filter

**Behavior**:
- When "All" selected: Show all masters and slaves (current behavior)
- When specific master selected: Show only that master and its connected slaves
- Empty state when no connections exist

**Features**:
- Smooth transitions when filter changes
- Maintain scroll position per filter (optional enhancement)
- Show breadcrumb or header indicating current filter

**Visual (Specific Master Selected)**:
```
┌──────────────────────────────────────────────────────┐
│  Viewing: Account A                      [× Clear]   │  ← Filter indicator
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌────────────────┐                                 │
│  │  Account A     │                                 │
│  │  (Master)      │────────────────┐                │
│  │  [Settings ⚙️] │                │                │
│  └────────────────┘                │                │
│                                    │                │
│                                    └─────▶ ┌──────────┐
│                                            │ Slave 1  │
│                                            │ Settings │
│                                            └──────────┘
│                                    ┌─────▶ ┌──────────┐
│                                    │       │ Slave 2  │
│                                    │       │ Settings │
│                                    │       └──────────┘
│                                    └─────▶ ┌──────────┐
│                                            │ Slave 4  │
│                                            │ Settings │
│                                            └──────────┘
└──────────────────────────────────────────────────────┘
```

### 3.3 Enhanced Settings Management

**Integration with Sidebar Approach**:

When a specific master is selected, the gear icon (⚙️) on the master card shows:
1. All copy settings originating from this master
2. Each setting displayed as: `Master → Slave` with delete button
3. Click on setting item → Opens detailed settings modal

**Settings List Display** (from AccountCardHeader gear icon):
```
┌─────────────────────────────────────┐
│ Copy Settings for Account A         │
├─────────────────────────────────────┤
│ Account A → Slave 1     [🗑️ Delete] │
│ • Lot: 1.0x                         │
│ • Reverse: No                       │
├─────────────────────────────────────┤
│ Account A → Slave 2     [🗑️ Delete] │
│ • Lot: 0.5x                         │
│ • Reverse: Yes                      │
├─────────────────────────────────────┤
│ Account A → Slave 4     [🗑️ Delete] │
│ • Lot: 2.0x                         │
│ • Reverse: No                       │
└─────────────────────────────────────┘
```

**Detailed Settings Modal** (click on a setting):
```
┌─────────────────────────────────────────────┐
│ Edit Copy Settings                    [×]   │
├─────────────────────────────────────────────┤
│ Master: Account A                           │
│ Slave:  Slave 1                             │
│                                             │
│ ┌─ Basic Settings ─────────────────────┐   │
│ │ Lot Multiplier:  [1.0    ] ×         │   │
│ │ Reverse Trade:   [ ] Enable           │   │
│ │ Enabled:         [✓] Active           │   │
│ └───────────────────────────────────────┘   │
│                                             │
│ ┌─ Advanced Filters ────────────────────┐   │
│ │ Allowed Symbols:                      │   │
│ │ [EURUSD, GBPUSD         ] [+ Add]     │   │
│ │                                       │   │
│ │ Blocked Symbols:                      │   │
│ │ [USDJPY                 ] [+ Add]     │   │
│ │                                       │   │
│ │ Magic Numbers:                        │   │
│ │ Allow: [12345, 67890    ] [+ Add]     │   │
│ │ Block: [99999           ] [+ Add]     │   │
│ └───────────────────────────────────────┘   │
│                                             │
│ ┌─ Symbol Mappings ─────────────────────┐   │
│ │ EURUSD → EURUSD.m       [🗑️]          │   │
│ │ GBPUSD → GBPUSD.m       [🗑️]          │   │
│ │ [+ Add Mapping]                       │   │
│ └───────────────────────────────────────┘   │
│                                             │
│          [Delete Settings]  [Save Changes]  │
└─────────────────────────────────────────────┘
```

---

## 4. User Workflows

### 4.1 View All Connections
1. Click "All Accounts" in sidebar (default state)
2. Main view shows all master-slave connections
3. Visual connections displayed with lines

### 4.2 Focus on Specific Master
1. Click "Account A" in sidebar
2. Sidebar item highlights
3. Main view filters to show only Account A and its slaves
4. Filter indicator appears at top ("Viewing: Account A")
5. Other masters and unrelated slaves hidden

### 4.3 Manage Settings for a Master
1. Select master in sidebar (e.g., "Account A")
2. Click gear icon (⚙️) on the master card
3. Settings list expands showing all connections from this master
4. Each setting shows: Target slave, basic info, delete button
5. Click on a setting → Opens detailed modal with all parameters
6. Edit parameters → Save
7. Or click delete → Confirm → Setting removed

### 4.4 Create New Connection
**Option A**: From Sidebar
1. Click "[+ Create Link]" button in sidebar
2. Dialog opens with master pre-selected (if filter active)
3. Choose slave, configure settings
4. Save

**Option B**: From Main View (existing)
1. Click "Create New Link" button in header
2. Dialog opens
3. Choose master, choose slave, configure
4. Save

### 4.5 Delete Connection
1. Select master in sidebar
2. Click gear icon on master card
3. Settings list expands
4. Click delete button (🗑️) next to the specific slave connection
5. Confirmation dialog appears
6. Confirm → Connection deleted, UI updates

---

## 5. State Management

### 5.1 Filter State
```typescript
interface FilterState {
  selectedMaster: string | 'all';  // 'all' or master account ID
}
```

**Location**: ConnectionsView component (parent state)
**Updates**: Via sidebar selection
**Effects**: Filters displayed accounts and connections

### 5.2 Derived Data
```typescript
// In ConnectionsView
const visibleMasters = selectedMaster === 'all'
  ? allMasters
  : allMasters.filter(m => m.id === selectedMaster);

const visibleSlaves = selectedMaster === 'all'
  ? allSlaves
  : allSlaves.filter(s =>
      copySettings.some(cs =>
        cs.master_account === selectedMaster &&
        cs.slave_account === s.id
      )
    );

const visibleConnections = selectedMaster === 'all'
  ? allCopySettings
  : allCopySettings.filter(cs => cs.master_account === selectedMaster);
```

---

## 6. Data Flow

```
┌─────────────────┐
│  API / Store    │
│  - Masters      │
│  - Slaves       │
│  - CopySettings │
└────────┬────────┘
         │
         ▼
┌──────────────────────────┐
│  ConnectionsView         │
│  State:                  │
│  - selectedMaster        │
│                          │
│  Derived:                │
│  - visibleMasters        │
│  - visibleSlaves         │
│  - visibleConnections    │
└─────┬──────────────┬─────┘
      │              │
      ▼              ▼
┌──────────┐   ┌─────────────────┐
│ Sidebar  │   │ Filtered View   │
│          │   │                 │
│ onClick  │   │ - Master Cards  │
│ ───────►│   │ - Slave Cards   │
│ update   │   │ - Connections   │
│ filter   │   │                 │
└──────────┘   └─────────────────┘
```

---

## 7. Animations & Transitions

### 7.1 Filter Change Animation
- **Duration**: 300ms
- **Easing**: ease-in-out
- **Behavior**:
  - Fade out hidden cards (opacity: 0)
  - Slide remaining cards to new positions
  - Fade in filter indicator

### 7.2 Settings Expansion
- **Duration**: 200ms
- **Easing**: ease-out
- **Behavior**:
  - Expand height from 0 to auto
  - Fade in content
  - Rotate gear icon 45° (already implemented)

### 7.3 Sidebar Collapse (Desktop)
- **Duration**: 250ms
- **Easing**: ease-in-out
- **Behavior**:
  - Sidebar width: 240px → 60px
  - Text fades out
  - Icons remain visible

### 7.4 Sidebar Drawer (Mobile)
- **Duration**: 300ms
- **Easing**: ease-out
- **Behavior**:
  - Slide in from left: translateX(-100%) → translateX(0)
  - Backdrop fade in: opacity: 0 → 0.5
  - Close: Reverse animation

---

## 8. Accessibility

### 8.1 Keyboard Navigation
- **Tab**: Navigate through sidebar items
- **Enter/Space**: Select master account
- **Arrow keys**: Navigate up/down in sidebar list
- **Escape**: Clear filter (return to "All"), or close sidebar (mobile)

### 8.2 ARIA Labels
```typescript
<nav aria-label="Master account filter">
  <button
    role="radio"
    aria-checked={isSelected}
    aria-label={`Filter by ${accountName}, ${connectionCount} connections`}
  >
    ...
  </button>
</nav>
```

### 8.3 Screen Reader Announcements
- On filter change: "Now showing connections for Account A"
- On clear filter: "Showing all connections"
- On settings expand: "Settings expanded, 3 connections"

---

## 9. Performance Considerations

### 9.1 Rendering Optimization
- Use `React.memo` for AccountCard components
- Memoize `visibleMasters`, `visibleSlaves`, `visibleConnections` with `useMemo`
- Virtualize sidebar list if > 50 masters (react-window)

### 9.2 Connection Line Rendering
- Only render connection lines for visible pairs
- Use Canvas API for > 20 connections (better performance)
- Consider hiding lines on filter change, show on animation complete

---

## 10. Edge Cases

### 10.1 No Connections
- Sidebar shows masters with "0 links"
- Main view shows empty state: "No connections configured"
- CTA: "Create your first connection"

### 10.2 Master with No Slaves
- Master appears in sidebar
- When selected, shows master card only
- Message: "No slave accounts connected"
- CTA: "Create New Link"

### 10.3 Orphaned Slaves
- Slaves not connected to any master
- Only visible in "All Accounts" view
- Consider: "Unconnected Accounts" section at bottom

### 10.4 Multiple Connections Between Same Pair
- Should not occur (DB constraint)
- If occurs: Show warning, allow user to delete duplicates

---

## 11. Future Enhancements

### Phase 2 (Post-MVP)
1. **Search in Sidebar**: Quick filter by account name
2. **Favorites**: Star frequently used masters for quick access
3. **Grouping**: Group masters by broker, strategy, or custom tags
4. **Batch Operations**: Select multiple connections, enable/disable all
5. **Connection Templates**: Save settings as templates for reuse

### Phase 3 (Advanced)
1. **Graph View**: Alternative visualization (nodes & edges)
2. **Analytics Panel**: Show copy stats per master (volume, profit, etc.)
3. **Drag & Drop**: Drag slave from one master to another (re-link)
4. **Multi-Master Filter**: Select multiple masters (checkbox mode)

---

## 12. Migration Path

To minimize disruption, implement in phases:

**Phase 1**: Add Sidebar (non-functional, shows all accounts)
**Phase 2**: Implement filter logic, update ConnectionsView
**Phase 3**: Add settings management integration
**Phase 4**: Polish animations, mobile optimization
**Phase 5**: Accessibility audit & testing

---

## 13. Success Metrics

Post-launch, measure:
1. **Task Completion Time**: How long to find and edit a specific connection
2. **Error Rate**: Misclicks, wrong connections edited
3. **User Satisfaction**: Subjective feedback on clarity
4. **Scalability**: Performance with 10, 20, 50+ masters

---

## Questions for Review

1. Should we persist the selected filter in localStorage?
2. Should "Create New Link" button in sidebar pre-select the filtered master?
3. How to handle master/slave role switching (if that's a feature)?
4. Do we need bulk edit/delete for connections?

---

**End of Design Document**
