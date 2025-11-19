# SANKEY Copier Web-UI: State Management Architecture Analysis

## Executive Summary
The web-ui codebase uses a **distributed, hook-based state management approach** with React Context for global state. Currently, there is NO Redux, Zustand, or Jotai. All state is managed through:
- React Context API (2 contexts)
- Custom hooks with local state (useState, useReducer)
- localStorage for persistence
- Direct component-level state

This architecture works for the current application size but shows clear pain points around state organization, prop drilling, and synchronization logic.

---

## 1. Current State Management Architecture

### 1.1 Global Contexts (2 total)

#### **SiteContext** (`lib/contexts/site-context.tsx`)
- **Purpose**: Multi-site management and API client provisioning
- **State**:
  - `sites: Site[]` - List of configured remote servers
  - `selectedSite: Site` - Currently selected server
  - `selectedSiteId: string` - ID of selected site
  - `isLoaded: boolean` - Hydration indicator
  - `apiClient: ApiClient` - Memoized API client
- **Operations**:
  - `addSite()`, `updateSite()`, `deleteSite()`, `selectSite()`
- **Persistence**: localStorage (keys: `sankey-copier-sites`, `sankey-copier-selected-site-id`)
- **Provider Location**: `app/[locale]/layout.tsx`

#### **SidebarContext** (`lib/contexts/sidebar-context.tsx`)
- **Purpose**: Layout and UI positioning state
- **State**:
  - `isOpen: boolean` - Sidebar open/closed
  - `isMobile: boolean` - Mobile viewport detected
  - `serverLogExpanded: boolean` - Server log visibility
  - `serverLogHeight: number` - Server log height in pixels
- **Operations**:
  - `setIsOpen()`, `setServerLogExpanded()`, `setServerLogHeight()`
- **Persistence**: localStorage (only sidebar-open state, conditional on desktop)
- **Provider Location**: `app/[locale]/layout.tsx`

---

### 1.2 Custom Hooks - Domain State (7 hooks)

#### **useSankeyCopier** (`hooks/useSankeyCopier.ts`) - CORE DOMAIN LOGIC
**Status**: Primary data management hook - Very complex

**State**:
```typescript
settings: CopySettings[]           // Copy trade settings
optimisticSettings: CopySettings[] // Optimistic UI updates
connections: EaConnection[]        // Connected accounts
loading: boolean
error: string | null
wsMessages: string[]               // WebSocket log messages
```

**Operations**:
- `toggleEnabled(id: number, status: number): Promise<void>` - Enable/disable copy
- `createSetting(data: CreateSettingsRequest): Promise<void>`
- `updateSetting(id: number, data: CopySettings): Promise<void>`
- `deleteSetting(id: number): Promise<void>`

**Special Features**:
- **Optimistic Updates**: Uses `useOptimistic` hook for instant UI feedback
- **WebSocket Connection**: Opens `ws://site-url/ws` for real-time updates
- **Auto-refetch**: Polls connections every 5 seconds
- **Error Handling**: Multiple error scenarios with user-friendly messages
- **Dependency**: Depends on SiteContext for selectedSite and apiClient

**Issues**:
- Complex dependency arrays
- Manual refetch after WebSocket messages
- State scattered across 4 separate useState calls
- Mixed concerns: API, WebSocket, optimization, error handling

---

#### **useSites** (`lib/hooks/use-sites.ts`)
**Status**: Site management - Moderate complexity

**State**:
```typescript
sites: Site[]           // All configured sites
selectedSiteId: string  // Current selection
isLoaded: boolean       // Hydration flag
```

**Operations**:
- `addSite(name, siteUrl): Site`
- `updateSite(id, updates): void`
- `deleteSite(id): void`
- `selectSite(id): void`

**Special Features**:
- Persists to localStorage on change
- Computes `selectedSite` via find/fallback logic
- Default site provided if none exist

---

#### **useAccountData** (`hooks/connections/useAccountData.ts`)
**Status**: Critical data transformation - Very complex

**State**:
```typescript
sourceAccounts: AccountInfo[]      // Master accounts
receiverAccounts: AccountInfo[]    // Slave accounts
```

**Operations**:
- `toggleSourceExpand(accountId)`, `toggleReceiverExpand(accountId)`

**Derived Computations**:
- Builds AccountInfo objects from connections + settings
- Calculates:
  - `isOnline`, `isTradeAllowed`, `isEnabled`, `isActive` states
  - `hasError`, `hasWarning` flags
  - Master online checks vs slave trade_allowed
  - All connected master status for each slave

**Complexity**: ~200 lines of effect logic, multiple passes over data

---

#### **useConnectionHighlight** (`hooks/connections/useConnectionHighlight.ts`)
**Status**: Hover/selection state management

**State**:
```typescript
hoveredSourceId: string | null     // Desktop hover
hoveredReceiverId: string | null
selectedSourceId: string | null    // Mobile selection
isMobile: boolean
```

**Operations**:
- `setHoveredSource()`, `setHoveredReceiver()`
- `handleSourceTap()`, `clearSelection()` (mobile)
- `isAccountHighlighted(id, type): boolean` - Complex logic

**Special Features**:
- Desktop uses hover; mobile uses tap selection
- Memoized connection mapping functions
- Bidirectional highlighting (source→receivers, receiver←sources)

---

#### **useAccountToggle** (`hooks/connections/useAccountToggle.ts`)
**Status**: Account toggle state + API orchestration

**Operations**:
- `toggleSourceEnabled(accountId, enabled)` - Updates local state + calls onToggle for all related settings
- `toggleReceiverEnabled(accountId, enabled)` - Same pattern

**Coupling**: Calls parent onToggle callback for each setting (potential API N+1 calls)

---

#### **useMasterFilter** (`hooks/useMasterFilter.ts`)
**Status**: Filter/derived state

**State**:
```typescript
selectedMaster: string | 'all'
```

**Derived**:
```typescript
visibleSourceAccounts: AccountInfo[]
visibleReceiverAccounts: AccountInfo[]
selectedMasterName: string | null
```

---

#### **useMtInstallations** (`hooks/useMtInstallations.ts`)
**Status**: MT4/MT5 installations management

**State**:
```typescript
installations: MtInstallation[]
loading: boolean
error: string | null
installing: string | null  // ID being installed
```

**Operations**:
- `fetchInstallations(): Promise<void>`
- `installToMt(id: string): Promise<{success, message}>`

---

### 1.3 Custom Hooks - UI/Utility (3 hooks)

#### **useSettingsValidation** (`hooks/useSettingsValidation.ts`)
**Status**: Pure computation hook (useMemo)
**Purpose**: Form validation for create/edit dialogs
**Returns**: `{isValid, errors[], warnings[]}`

#### **useToast** (`hooks/use-toast.ts`)
**Status**: Custom implementation using useReducer + external dispatch
**Pattern**: Similar to Radix toast implementation
**State Persistence**: In-memory (not persistent across page reloads)

#### **useServerLogs** (`components/ServerLog.hooks.ts`)
**Status**: Server log management
**State**: 
- logs, isLoading, error, autoRefresh
**Operations**: fetchLogs (with polling support)

#### **useLogViewerResize** (`components/ServerLog.hooks.ts`)
**Status**: UI resize state
**State**: height, isMaximized, isResizing

#### **useLogViewerLayout** (`components/ServerLog.hooks.ts`)
**Status**: Layout effect hook
**Purpose**: DOM manipulation for log viewer layout

---

### 1.4 Custom Hooks - Visualization (2 hooks)

#### **useFlowData** (`hooks/useFlowData.ts`)
**Status**: React Flow visualization
**Purpose**: Transforms AccountInfo + settings into React Flow nodes + edges
**Input**: 21 parameters
**Output**: Memoized nodes and edges
**Complexity**: Large parameter list, many callbacks passed through

#### **useSVGConnections** (`hooks/connections/useSVGConnections.ts`)
**Status**: Direct SVG manipulation
**Purpose**: Draws SVG connection lines between accounts
**Special**: ResizeObserver + window resize listener, manual DOM manipulation

#### **useAccountRefs** (`hooks/connections/useAccountRefs.ts`)
**Status**: DOM ref management
**Purpose**: Stores refs to account card elements for positioning

---

## 2. State Type Definitions

### Core Domain Types (`types/index.ts`)

```typescript
interface CopySettings {
  id: number
  status: number                    // 0=OFF, 1=ON
  master_account: string
  slave_account: string
  lot_multiplier: number | null
  reverse_trade: boolean
  symbol_mappings: SymbolMapping[]
  filters: TradeFilters
}

interface EaConnection {
  account_id: string
  ea_type: 'Master' | 'Slave'
  platform: 'MT4' | 'MT5'
  account_number: number
  broker: string
  account_name: string
  server: string
  balance: number
  equity: number
  currency: string
  leverage: number
  last_heartbeat: string
  status: 'Online' | 'Offline' | 'Timeout'
  connected_at: string
  open_positions?: number
  is_trade_allowed: boolean
  // Legacy fields
  role?: 'master' | 'slave'
  is_online?: boolean
}

interface AccountInfo {
  id: string
  name: string
  isOnline: boolean
  isEnabled: boolean        // User's switch state
  isActive: boolean         // Calculated: ready for trading
  hasError: boolean
  hasWarning: boolean
  errorMsg: string
  isExpanded: boolean       // UI state for card expansion
}
```

### UI Types
```typescript
interface Site {
  id: string
  name: string
  siteUrl: string
}

interface MtInstallation {
  id: string
  name: string
  type: MtType           // 'MT4' | 'MT5'
  platform: Architecture // '32-bit' | '64-bit'
  path: string
  executable: string
  version: string | null
  components: InstalledComponents
}
```

---

## 3. Data Flow Architecture

### Main Entry Point: `app/[locale]/connections/page.tsx`

```
ConnectionsPage
├── useSiteContext()           → selectedSite, apiClient
├── useSidebar()              → layout state
├── useSankeyCopier()         → settings, connections, operations
└── <ConnectionsViewReactFlow>
    ├── useAccountData()      → sourceAccounts, receiverAccounts
    ├── useConnectionHighlight() → hover/selection state
    ├── useAccountToggle()    → toggle functions
    ├── useMasterFilter()     → filtered accounts
    ├── useFlowData()         → nodes, edges
    ├── <MasterAccountFilter> → selected master
    ├── <CreateConnectionDialog>
    │   ├── useSettingsValidation()
    │   └── onCreate() callback
    ├── <EditCopySettingsDialog>
    │   ├── useSettingsValidation()
    │   └── onUpdate() callback
    └── <ReactFlow visualization>
        └── useSVGConnections() → SVG drawing + resize handling
```

### State Updates Flow

**User Creates Connection**:
```
User clicks "Create" 
→ Dialog opens, local form state
→ User submits
→ useSankeyCopier().createSetting()
  → Optimistic update (useOptimistic)
  → POST to /settings
  → Refetch all settings on success
→ UI re-renders with optimistic state
```

**WebSocket Update**:
```
Rust server sends "settings_*" message
→ ws.onmessage() handler fires
→ useSankeyCopier().fetchSettings() called
→ setState(newSettings)
→ useMasterFilter recomputes visibleAccounts
→ useAccountData recomputes AccountInfo
→ Components re-render
```

**Toggle Account Enable**:
```
User clicks toggle switch
→ useAccountToggle.toggleSourceEnabled(accountId, enabled)
→ Local state updates immediately
→ For each related setting: onToggle(settingId, status)
→ useSankeyCopier.toggleEnabled() called (N calls for N settings)
  → Optimistic update
  → POST to /settings/{id}/toggle
  → Refetch all settings
→ UI updates
```

---

## 4. Component State Patterns

### Pattern 1: Props Drilling
```typescript
// ConnectionsViewReactFlow receives 5 props
// Passes down handlers to child components
// useFlowData receives 21 parameters!

interface UseFlowDataProps {
  sourceAccounts: AccountInfo[]
  receiverAccounts: AccountInfo[]
  settings: CopySettings[]
  getAccountConnection: (accountId: string) => EaConnection | undefined
  getAccountSettings: (accountId: string, type: 'source' | 'receiver') => CopySettings[]
  toggleSourceExpand: (id: string) => void
  toggleReceiverExpand: (id: string) => void
  toggleSourceEnabled: (id: string, enabled: boolean) => void
  toggleReceiverEnabled: (id: string, enabled: boolean) => void
  handleEditSetting: (setting: CopySettings) => void
  handleDeleteSetting: (setting: CopySettings) => void
  hoveredSourceId: string | null
  hoveredReceiverId: string | null
  selectedSourceId: string | null
  isAccountHighlighted: (accountId: string, type: 'source' | 'receiver') => boolean
  isMobile: boolean
  content: any  // i18n content
}
```

### Pattern 2: localStorage Persistence
Found in:
- `useSites()` - Loads on mount, saves on change
- `sidebar-context` - Saves open/closed state
- Manual `localStorage.getItem/setItem` calls (14 instances)

No centralized persistence layer.

### Pattern 3: Derived State Chains
```
EaConnection + CopySettings
  ↓ (useAccountData)
AccountInfo[]
  ↓ (useMasterFilter)
visibleSourceAccounts, visibleReceiverAccounts
  ↓ (useFlowData)
Node[], Edge[] (React Flow)
  ↓ (useSVGConnections)
SVG DOM elements
```

Multiple transformation steps, each with its own useState.

### Pattern 4: WebSocket + Polling Hybrid
```
useSankeyCopier():
- Opens WebSocket on mount
- Sets up interval to poll connections every 5s
- On WebSocket message: refetch settings
- No unified synchronization mechanism
```

---

## 5. localStorage Usage (14 instances)

| Location | Keys | Purpose | Pattern |
|----------|------|---------|---------|
| `sidebar-context` | `sidebar-open` | Sidebar persistence | Conditional (desktop only) |
| `use-sites` | `sankey-copier-sites`, `sankey-copier-selected-site-id` | Site management | Two separate calls |
| Various dialogs | Various dialog open states | Dialog state persistence | Scattered |

**Pain Point**: No unified persistence strategy or abstraction.

---

## 6. Current Pain Points & Anti-Patterns

### 6.1 **Prop Drilling**
- `useFlowData` receives 21 parameters
- Components pass handlers through 3-4 levels
- Hard to track which props are used where
- Makes refactoring dangerous

### 6.2 **State Fragmentation**
- Settings managed in `useSankeyCopier`
- Account state in `useAccountData`
- Highlight state in `useConnectionHighlight`
- Filter state in `useMasterFilter`
- No single source of truth

**Example**: Settings can change in useSankeyCopier but AccountInfo computed in useAccountData - requires manual synchronization via dependencies

### 6.3 **Complex Dependency Arrays**
```typescript
// useSankeyCopier useEffect
useEffect(() => {
  fetchSettings()
  fetchConnections()
  const interval = setInterval(fetchConnections, 5000)
  return () => clearInterval(interval)
}, [fetchSettings, fetchConnections])

// useAccountData useEffect - 7+ dependencies
useEffect(() => { ... }, [
  settings, 
  connections, 
  content.allSourcesInactive, 
  content.someSourcesInactive, 
  content.autoTradingDisabled
])
```

Difficult to debug stale closures and missing dependencies.

### 6.4 **Manual Synchronization**
```typescript
// WebSocket message triggers manual refetch
ws.onmessage = (event) => {
  if (message.startsWith('settings_')) {
    fetchSettings()  // Manual refetch - no automatic sync
  }
}
```

No pub/sub mechanism. Needs manual coordination.

### 6.5 **Computed State Chains**
```
settings → accountData → filtered accounts → flow nodes → SVG
```

Each step is separate hook, no normalized state.

### 6.6 **Missing Optimizations**
- No memoization of hook return values (not using useMemo for complex computations)
- Callback functions recreated on every render if dependencies missing
- No selector pattern to prevent unnecessary re-renders

### 6.7 **API Orchestration Issues**
```typescript
// When toggling account, calls onToggle N times (one per setting)
sourceSettings.forEach((setting) => {
  onToggle(setting.id, setting.status)  // Potential N API calls
})

// In useSankeyCopier:
await apiClient.post(`/settings/${id}/toggle`, ...)
fetchSettings()  // Full refetch after every single toggle
```

Inefficient - could batch updates.

### 6.8 **localStorage Scattered**
- Multiple files manually calling localStorage
- No unified abstraction
- No type safety
- Hard to refactor storage strategy

### 6.9 **UI State Mixed with Domain State**
- Expansion state (isExpanded) mixed in AccountInfo
- Mobile detection in multiple hooks (useConnectionHighlight, useFlowData)
- No clear separation of concerns

### 6.10 **Error Handling**
- Errors handled inconsistently
- No error boundary or centralized error logging
- User sees different error messages for same failure
- No retry mechanism

---

## 7. Areas Perfect for Jotai Integration

### 7.1 **Atomic State Structure** (Jotai's Strength)
```
Atoms:
├── sitesAtom[]
├── selectedSiteIdAtom
├── settingsAtom[]
├── connectionsAtom[]
├── wsMessagesAtom[]
├── selectedMasterAtom
├── hoveredSourceAtom
├── hoveredReceiverAtom
├── selectedSourceAtom
├── sidebarOpenAtom
├── serverLogExpandedAtom
├── serverLogHeightAtom
├── accountsAtom (derived)
├── visibleAccountsAtom (derived)
└── flowDataAtom (derived)

Derived atoms:
├── sourceAccountsAtom = atom((get) => computeFromSettings())
├── receiverAccountsAtom = atom((get) => computeFromSettings())
├── visibleSourceAtom = atom((get) => filter(sourceAtom, selectedMasterAtom))
└── accountsHighlightedAtom = atom((get) => compute(hoverAtom, selectedAtom))
```

**Benefits**:
- Clear atomic structure
- No prop drilling
- Automatic dependency tracking
- Built-in derived state (selectors)

### 7.2 **localStorage Integration**
Jotai's `atomWithStorage`:
```typescript
const sitesAtom = atomWithStorage('sankey-sites', [])
const sidebarOpenAtom = atomWithStorage('sidebar-open', false)
```

Replaces manual localStorage management throughout codebase.

### 7.3 **Async Data & WebSocket**
Jotai's `atomWithAsyncData` pattern:
```typescript
const settingsAtom = atom(async (get) => {
  const site = get(siteAtom)
  return apiClient.get('/settings')
})

const wsAtom = atom((get) => {
  const site = get(siteAtom)
  const ws = new WebSocket(`ws://${site.siteUrl}/ws`)
  ws.onmessage = () => {
    // Trigger settings refetch
  }
})
```

### 7.4 **Optimistic Updates**
```typescript
const toggleSettingAtom = atom(
  null,
  async (get, set, id: number) => {
    // Optimistic update
    set(settingsAtom, prev => 
      prev.map(s => s.id === id ? {...s, status: !s.status} : s)
    )
    
    // Async operation
    try {
      await apiClient.toggle(id)
      // Refetch on success
      set(settingsAtom, async (get) => apiClient.get('/settings'))
    } catch {
      // Revert on error - set will handle previous state
    }
  }
)
```

### 7.5 **Batch Operations**
```typescript
const batchToggleAccountAtom = atom(
  null,
  async (get, set, accountId: string) => {
    const settings = get(settingsAtom)
    const relatedSettings = settings.filter(s => s.master_account === accountId)
    
    // Single atom update with all changes
    set(settingsAtom, prev =>
      prev.map(s => 
        relatedSettings.find(rs => rs.id === s.id)
          ? {...s, status: !s.status}
          : s
      )
    )
    
    // Batch API call
    await Promise.all(relatedSettings.map(s => apiClient.toggle(s.id)))
  }
)
```

### 7.6 **No More Prop Drilling**
```typescript
// Instead of:
interface UseFlowDataProps {
  sourceAccounts: AccountInfo[]
  // ... 20 more props
}

// Simply:
const useFlowData = () => {
  const sourceAccounts = useAtomValue(sourceAccountsAtom)
  const receiverAccounts = useAtomValue(receiverAccountsAtom)
  const hoveredSource = useAtomValue(hoveredSourceAtom)
  // ... read exactly what you need
  
  const setEditingSettings = useSetAtom(editingSettingsAtom)
  // ... write what you need
}
```

### 7.7 **Automatic Invalidation & Refetch**
```typescript
// No manual dependency arrays
const settingsAtom = atom(async (get) => {
  const site = get(siteAtom)  // Jotai tracks dependency
  const wsMessages = get(wsMessagesAtom)  // Automatic re-fetch when messages change
  return await apiClient.get('/settings')
})
```

### 7.8 **Better DevTools**
- Jotai DevTools can inspect all atoms and their values
- Time-travel debugging
- Clear visualization of dependencies

---

## 8. Existing Anti-Patterns Jotai Would Fix

| Issue | Current | Jotai |
|-------|---------|-------|
| **Prop drilling** | useFlowData(21 params) | useAtomValue() directly |
| **Storage** | Manual localStorage in 3 places | atomWithStorage() |
| **Derived state** | Multiple useState + useMemo | Derived atoms (automatic) |
| **Dependencies** | Manual arrays, easy to break | Automatic tracking via get() |
| **WebSocket sync** | Manual refetch in onmessage | Atom dependency updates automatically |
| **Error handling** | Scattered setError() calls | Atom can contain error state |
| **Optimization** | useCallback hell | Atoms naturally prevent unnecessary renders |
| **Testing** | Mock contexts + providers | Direct atom access, no providers needed |
| **DevTools** | None | Jotai DevTools with time-travel |

---

## 9. Key State Entities Summary

### High-Level Entity Diagram

```
User Config
  ├── Sites (global)
  │   └── Selected Site
  │       └── API Client
  │
  UI Layout
  │   ├── Sidebar open/closed
  │   └── Server log state
  │
  Domain Data
  │   ├── Copy Settings ← Fetched & cached
  │   ├── EA Connections ← Polled every 5s
  │   └── WebSocket Messages ← Real-time
  │
  Computed State
  │   ├── Account Data (master/slave separation)
  │   ├── Master Filter (visible accounts)
  │   └── Flow Visualization (nodes/edges)
  │
  Interaction State
  │   ├── Hover highlights
  │   ├── Selection (mobile)
  │   ├── Dialog open/close
  │   └── Editing settings
  │
  UI Display
      ├── Toast messages
      ├── Server logs
      └── SVG connections
```

---

## 10. Migration Path to Jotai (High-Level)

### Phase 1: Setup & Global State
1. Add jotai dependency
2. Create atoms for SiteProvider (sites, selectedSite)
3. Create atoms for SidebarProvider (sidebar state)
4. Create atoms for toast state
5. Create root JotaiProvider

### Phase 2: Core Domain State
1. Create atoms for useSankeyCopier data (settings, connections, wsMessages)
2. Implement WebSocket with atom dependency
3. Implement optimistic updates using write atoms
4. Replace useSankeyCopier hook

### Phase 3: Connection View State
1. Create atoms for account data (sourceAccounts, receiverAccounts)
2. Create atoms for highlighting (hovered, selected)
3. Create atoms for filtering (selectedMaster)
4. Create derived atoms for filtered accounts
5. Replace multiple hooks with atomic structure

### Phase 4: Dialog & Form State
1. Create atoms for dialog states (createDialogOpen, editingSettings)
2. Create atoms for form validation
3. Replace local component state

### Phase 5: Optimization
1. Implement selective subscriptions
2. Add Jotai DevTools
3. Benchmark and optimize atom structure
4. Remove unused hooks

---

## 11. Comparison: Current vs Jotai Approach

### Current (Hook-based):
```typescript
// Component
const { settings, connections, loading } = useSankeyCopier()
const { sourceAccounts, receiverAccounts } = useAccountData({
  connections, settings, content: {...}
})
const { selectedMaster, setSelectedMaster, visibleSourceAccounts } = useMasterFilter({
  connections, settings, sourceAccounts, receiverAccounts
})
const { nodes, edges } = useFlowData({
  sourceAccounts, receiverAccounts, settings,
  getAccountConnection: ...,
  getAccountSettings: ...,
  toggleSourceExpand: ...,
  // ... 15 more props
})
```

### Jotai (Atomic):
```typescript
// Component
const settings = useAtomValue(settingsAtom)
const connections = useAtomValue(connectionsAtom)
const sourceAccounts = useAtomValue(sourceAccountsAtom)  // Already derived
const visibleSourceAccounts = useAtomValue(visibleSourceAccountsAtom)  // Automatic
const nodes = useAtomValue(nodesAtom)  // Automatic from derivation chain
const edges = useAtomValue(edgesAtom)

const setSelectedMaster = useSetAtom(selectedMasterAtom)
const toggleSourceExpand = useSetAtom(toggleSourceExpandAtom)  // Action atom
```

**Cleaner, more maintainable, zero prop drilling.**

---

## Key Metrics

- **Total React Files**: 103
- **Custom Hooks**: 11 (7 domain, 3 UI, 1 visualization)
- **Global Contexts**: 2 (could become 1 with Jotai provider)
- **localStorage Usage Points**: 14
- **Largest Component Props**: 21 (useFlowData)
- **Complex Dependency Arrays**: 10+
- **Polling Intervals**: 1 (5-second connections poll)
- **WebSocket Connections**: 1 per site
- **Type-Safe State**: Medium (some implicit dependencies)

---

## Conclusion

The current state management works but shows clear architectural strain:

1. **State is fragmented** across many hooks with implicit dependencies
2. **Prop drilling** reaches 21 parameters in some functions
3. **Synchronization is manual** (WebSocket message → manual refetch)
4. **Persistence is scattered** across files
5. **Derived state chains** are opaque and hard to trace
6. **No optimization story** (prop drilling prevents selective updates)

**Jotai would provide**:
- Atomic state structure (clear, single source of truth)
- Automatic dependency management
- Built-in storage persistence
- Natural handling of async/WebSocket
- Complete elimination of prop drilling
- Better DevTools and debugging
- Simpler testing (no provider wrapping needed)

This codebase is at the sweet spot for Jotai integration - complex enough to benefit greatly, not so complex that migration would be risky.

