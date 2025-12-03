import { render } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { StatusIndicatorBar } from '@/components/connections/StatusIndicatorBar';
import type { AccountInfo } from '@/types';

const baseAccount: AccountInfo = {
  id: 'mock-account',
  name: 'Mock Broker_0001',
  accountType: 'master',
  isOnline: true,
  isEnabled: true,
  isActive: true,
  hasError: false,
  hasWarning: false,
  errorMsg: '',
  isExpanded: false,
};

function createAccount(overrides: Partial<AccountInfo>): AccountInfo {
  return { ...baseAccount, ...overrides };
}

describe('StatusIndicatorBar', () => {
  it('renders green for runtimeStatus=2', () => {
    const { container } = render(
      <StatusIndicatorBar account={createAccount({ runtimeStatus: 2 })} type="source" />
    );
    expect(container.firstChild).toHaveClass('bg-green-500');
  });

  it('renders amber for runtimeStatus=1', () => {
    const { container } = render(
      <StatusIndicatorBar account={createAccount({ runtimeStatus: 1 })} type="source" />
    );
    expect(container.firstChild).toHaveClass('bg-amber-500');
  });

  it('renders gray for runtimeStatus=0', () => {
    const { container } = render(
      <StatusIndicatorBar account={createAccount({ runtimeStatus: 0, isActive: false })} type="source" />
    );
    expect(container.firstChild).toHaveClass('bg-gray-300');
  });

  it('falls back to green when runtimeStatus undefined but account is active', () => {
    const { container } = render(
      <StatusIndicatorBar account={createAccount({ runtimeStatus: undefined, isActive: true })} type="source" />
    );
    expect(container.firstChild).toHaveClass('bg-green-500');
  });

  it('prefers warning color even if runtimeStatus is CONNECTED', () => {
    const { container } = render(
      <StatusIndicatorBar account={createAccount({ hasWarning: true, runtimeStatus: 2 })} type="source" />
    );
    expect(container.firstChild).toHaveClass('bg-yellow-500');
  });
});
