import { test, expect } from '@playwright/test';
import type { Locator, Page, Route } from '@playwright/test';
import type { CopySettings, TradeGroup, TradeGroupMember } from '@/types';
import { gotoApp } from './helpers/navigation';
import { mockConnections, mockSettings } from './mocks/testData';

type TradeGroupsState = Map<string, TradeGroup>;
type MembersState = Map<string, TradeGroupMember[]>;

interface ToggleCall {
  master: string;
  slave: string;
  body: unknown;
}

const TARGET_SLAVE = 'FxPro_55555007';
const TARGET_MASTER = 'XM_11111003';

function buildApiState(settings: CopySettings[]) {
  const tradeGroups: TradeGroupsState = new Map();
  const members: MembersState = new Map();
  const now = new Date().toISOString();

  settings.forEach((setting) => {
    if (!tradeGroups.has(setting.master_account)) {
      tradeGroups.set(setting.master_account, {
        id: setting.master_account,
        master_settings: {
          enabled: true,
          symbol_prefix: null,
          symbol_suffix: null,
          config_version: 1,
        },
        master_runtime_status: 2,
        created_at: now,
        updated_at: now,
      });
    }

    const member: TradeGroupMember = {
      id: setting.id,
      trade_group_id: setting.master_account,
      slave_account: setting.slave_account,
      slave_settings: {
        lot_calculation_mode: setting.lot_calculation_mode ?? 'multiplier',
        lot_multiplier: setting.lot_multiplier ?? 1,
        reverse_trade: setting.reverse_trade ?? false,
        symbol_prefix: setting.symbol_prefix ?? null,
        symbol_suffix: setting.symbol_suffix ?? null,
        symbol_mappings: setting.symbol_mappings ?? [],
        filters:
          setting.filters ?? {
            allowed_symbols: null,
            blocked_symbols: null,
            allowed_magic_numbers: null,
            blocked_magic_numbers: null,
          },
        config_version: 1,
        source_lot_min: setting.source_lot_min ?? null,
        source_lot_max: setting.source_lot_max ?? null,
        sync_mode: setting.sync_mode,
        limit_order_expiry_min: setting.limit_order_expiry_min,
        market_sync_max_pips: setting.market_sync_max_pips,
        max_slippage: setting.max_slippage,
        copy_pending_orders: setting.copy_pending_orders,
        max_retries: setting.max_retries,
        max_signal_delay_ms: setting.max_signal_delay_ms,
        use_pending_order_for_delayed: setting.use_pending_order_for_delayed,
      },
      status: setting.status ?? 0,
      runtime_status: setting.runtime_status ?? setting.status ?? 0,
      enabled_flag: setting.enabled_flag ?? (setting.status !== 0),
      created_at: now,
      updated_at: now,
    };

    const list = members.get(setting.master_account) ?? [];
    list.push(member);
    members.set(setting.master_account, list);
  });

  tradeGroups.forEach((group, masterId) => {
    const memberList = members.get(masterId) ?? [];
    const hasEnabledMember = memberList.some((m) => m.enabled_flag);
    const highestRuntime = memberList.reduce((acc, m) => Math.max(acc, m.runtime_status ?? 0), 0);
    group.master_settings.enabled = hasEnabledMember;
    group.master_runtime_status = highestRuntime;
  });

  return { tradeGroups, members };
}

function extractSegment(url: string, marker: string): string {
  const pathname = new URL(url).pathname;
  const segments = pathname.split('/').filter(Boolean);
  const index = segments.findIndex((segment) => segment === marker);
  if (index === -1 || !segments[index + 1]) {
    return '';
  }
  return decodeURIComponent(segments[index + 1]);
}

async function setupMockWebSocket(page: Page) {
  await page.addInitScript(() => {
    class MockWebSocket {
      static sockets: MockWebSocket[] = [];
      readyState = 0;
      onopen: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;

      constructor(public url: string) {
        MockWebSocket.sockets.push(this);
        setTimeout(() => {
          this.readyState = 1;
          this.onopen?.(new Event('open'));
        }, 10);
      }

      send() {}

      close() {
        this.readyState = 3;
        this.onclose?.({} as CloseEvent);
      }
    }

    (window as unknown as { __emitWsMessage?: (message: string) => void }).__emitWsMessage = (message: string) => {
      MockWebSocket.sockets.forEach((socket) => {
        socket.onmessage?.({ data: message } as MessageEvent);
      });
    };

    (window as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;
  });
}

function updateMemberRuntime(
  membersState: MembersState,
  masterAccount: string,
  slaveAccount: string,
  runtimeStatus: number,
) {
  const memberList = membersState.get(masterAccount);
  if (!memberList) {
    return;
  }

  const target = memberList.find((member) => member.slave_account === slaveAccount);
  if (!target) {
    return;
  }

  target.runtime_status = runtimeStatus;
  target.status = runtimeStatus;
}

async function waitForReactHydration(locator: Locator) {
  await expect
    .poll(
      async () =>
        locator.evaluate((el) =>
          Object.getOwnPropertyNames(el).some((key) => key.startsWith('__reactFiber$')),
        ),
      { timeout: 5000 },
    )
    .toBe(true);
}

async function fulfillJson(route: Route, data: unknown) {
  await route.fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify(data),
  });
}

test.describe('Runtime vs intent toggles', () => {
  test('runtime badge falls back to Manual OFF when MT auto-trading is disabled', async ({ page }) => {
    const targetMaster = 'FxPro_12345001';
    const targetSlave = 'FxPro_22222004';
    const { tradeGroups, members } = buildApiState(mockSettings);

    const connections = mockConnections.map((connection) => ({ ...connection }));
    const targetConnection = connections.find((conn) => conn.account_id === targetMaster);
    if (targetConnection) {
      targetConnection.is_trade_allowed = false;
    }
    const slaveConnection = connections.find((conn) => conn.account_id === targetSlave);
    if (slaveConnection) {
      slaveConnection.is_trade_allowed = false;
    }

    await page.route('**/api/connections', async (route) => {
      await fulfillJson(route, connections);
    });

    await page.route('**/api/trade-groups/*/members', async (route) => {
      const master = extractSegment(route.request().url(), 'trade-groups');
      await fulfillJson(route, members.get(master) ?? []);
    });

    await page.route('**/api/trade-groups', async (route) => {
      await fulfillJson(route, Array.from(tradeGroups.values()));
    });

    await gotoApp(page);

    const masterCard = page.locator(`[data-account-id="${targetMaster}"]`).first();
    await expect(masterCard).toBeVisible();

    await expect(masterCard.getByText('Streaming')).toHaveCount(0);
    await expect(masterCard.getByText('Manual OFF')).toBeVisible();

    const memberCard = page.locator(`[data-account-id="${targetSlave}"]`).first();
    await expect(memberCard).toBeVisible();
    await expect(memberCard.getByText('Receiving')).toHaveCount(0);
    await expect(memberCard.getByText('Manual OFF')).toBeVisible();
  });

  test('runtime badge updates only after WebSocket refresh', async ({ page }) => {
    const { tradeGroups, members } = buildApiState(mockSettings);
    const toggleCalls: ToggleCall[] = [];

    await setupMockWebSocket(page);

    await page.route('**/api/connections', async (route) => {
      await fulfillJson(route, mockConnections);
    });

    await page.route('**/api/trade-groups/*/members/*/toggle', async (route) => {
      const master = extractSegment(route.request().url(), 'trade-groups');
      const slave = extractSegment(route.request().url(), 'members');
      const body = route.request().postDataJSON();
      toggleCalls.push({ master, slave, body });

      const memberList = members.get(master);
      if (memberList) {
        const target = memberList.find((member) => member.slave_account === slave);
        if (target && typeof body?.enabled === 'boolean') {
          target.enabled_flag = body.enabled;
        }
      }

      await route.fulfill({ status: 200, body: '' });
    });

    await page.route('**/api/trade-groups/*/members', async (route) => {
      const master = extractSegment(route.request().url(), 'trade-groups');
      await fulfillJson(route, members.get(master) ?? []);
    });

    await page.route('**/api/trade-groups', async (route) => {
      await fulfillJson(route, Array.from(tradeGroups.values()));
    });

    await gotoApp(page);

    const receiverCard = page.locator(`[data-account-id="${TARGET_SLAVE}"]`).first();
    await expect(receiverCard).toBeVisible();

    await expect(receiverCard.getByText('Manual OFF')).toBeVisible();

    const toggleSwitch = receiverCard.getByTestId('account-toggle-switch');

    // Ensure React has hydrated the switch before interacting so event handlers exist
    await waitForReactHydration(toggleSwitch);
    const toggleHandle = await toggleSwitch.elementHandle();
    if (!toggleHandle) {
      throw new Error('Toggle switch element not found');
    }

    await page.waitForFunction(
      (el) => {
        const rect = el.getBoundingClientRect();
        return (
          rect.top >= 0 &&
          rect.left >= 0 &&
          rect.bottom <= window.innerHeight &&
          rect.right <= window.innerWidth
        );
      },
      toggleHandle
    );

    await toggleSwitch.click({ force: true });

    await expect
      .poll(() => toggleCalls.length, { timeout: 7000 })
      .toBe(1);
    expect(toggleCalls[0]).toMatchObject({
      master: TARGET_MASTER,
      slave: TARGET_SLAVE,
      body: { enabled: true },
    });

    await expect(receiverCard.getByText('Manual OFF')).toBeVisible();
    await expect(toggleSwitch).toHaveAttribute('data-pending', 'true');

    updateMemberRuntime(members, TARGET_MASTER, TARGET_SLAVE, 2);
    const masterGroup = tradeGroups.get(TARGET_MASTER);
    if (masterGroup) {
      masterGroup.master_runtime_status = 2;
    }
    await page.evaluate(() => {
      (window as unknown as { __emitWsMessage?: (message: string) => void }).__emitWsMessage?.('member_runtime_update');
    });

    await expect(receiverCard.getByText('Receiving')).toBeVisible();
    await expect.poll(async () => toggleSwitch.getAttribute('data-pending')).toBeNull();
  });
});
