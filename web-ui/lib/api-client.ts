/**
 * API Client for communicating with Rust Server
 *
 * This client makes direct requests to the Rust server based on the selected Site.
 * It bypasses the Next.js API proxy to support multi-site selection per browser.
 */

import type { Site } from './types/site';

/**
 * RFC 9457 Problem Details structure
 * https://www.rfc-editor.org/rfc/rfc9457.html
 */
interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  instance?: string;
}

/**
 * Check if response is RFC 9457 Problem Details
 */
function isProblemDetails(data: unknown): data is ProblemDetails {
  return (
    typeof data === 'object' &&
    data !== null &&
    'type' in data &&
    'title' in data &&
    'status' in data
  );
}

/**
 * API Client class
 */
export class ApiClient {
  private baseUrl: string;

  constructor(site: Site) {
    this.baseUrl = site.siteUrl;
  }

  /**
   * Handle response and extract error details from RFC 9457 Problem Details
   */
  private async handleResponse<T>(response: Response): Promise<T> {
    // Handle successful responses (2xx)
    if (response.ok) {
      // HTTP 204 No Content and 205 Reset Content have no response body
      if (response.status === 204 || response.status === 205) {
        return undefined as T;
      }

      // Parse JSON for other successful responses
      try {
        const data = await response.json();
        return data as T;
      } catch (error) {
        // If JSON parsing fails for a successful response, return undefined
        return undefined as T;
      }
    }

    // Handle error responses (4xx, 5xx)
    let data: unknown;
    try {
      data = await response.json();
    } catch (error) {
      // If JSON parsing fails, throw a generic error
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    // Check for RFC 9457 Problem Details
    if (isProblemDetails(data)) {
      const errorMsg = data.detail || data.title || `Server error: ${data.status}`;
      throw new Error(errorMsg);
    }

    // Fallback to generic error
    throw new Error(`Server returned ${response.status}: ${response.statusText}`);
  }

  /**
   * Make a GET request
   */
  async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}/api${path}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Make a POST request
   */
  async post<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}/api${path}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: body ? JSON.stringify(body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Make a PUT request
   */
  async put<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}/api${path}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: body ? JSON.stringify(body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Make a DELETE request
   */
  async delete<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}/api${path}`, {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Make a PATCH request
   */
  async patch<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}/api${path}`, {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
      },
      body: body ? JSON.stringify(body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  // ============================================================================
  // TradeGroups API (Master settings)
  // ============================================================================

  /**
   * List all TradeGroups (Master accounts with settings)
   */
  async listTradeGroups(): Promise<import('@/types').TradeGroup[]> {
    return this.get('/trade-groups');
  }

  /**
   * Get a specific TradeGroup by master account ID
   */
  async getTradeGroup(masterAccount: string): Promise<import('@/types').TradeGroup> {
    return this.get(`/trade-groups/${encodeURIComponent(masterAccount)}`);
  }

  /**
   * Update Master settings for a TradeGroup
   */
  async updateTradeGroupSettings(
    masterAccount: string,
    settings: import('@/types').MasterSettings
  ): Promise<void> {
    return this.put(`/trade-groups/${encodeURIComponent(masterAccount)}`, settings);
  }

  /**
   * Toggle Master enabled state
   */
  async toggleMaster(
    masterAccount: string,
    enabled: boolean
  ): Promise<import('@/types').TradeGroup> {
    return this.post(`/trade-groups/${encodeURIComponent(masterAccount)}/toggle`, { enabled });
  }

  // ============================================================================
  // TradeGroupMembers API (Slave settings)
  // ============================================================================

  /**
   * List all members for a TradeGroup
   */
  async listTradeGroupMembers(masterAccount: string): Promise<import('@/types').TradeGroupMember[]> {
    return this.get(`/trade-groups/${encodeURIComponent(masterAccount)}/members`);
  }

  /**
   * Add a new member to a TradeGroup
   */
  async addTradeGroupMember(
    masterAccount: string,
    data: { slave_account: string; slave_settings: import('@/types').SlaveSettings; status: number }
  ): Promise<import('@/types').TradeGroupMember> {
    return this.post(`/trade-groups/${encodeURIComponent(masterAccount)}/members`, data);
  }

  /**
   * Update a TradeGroup member's settings
   * Server expects SlaveSettings directly (not wrapped)
   */
  async updateTradeGroupMember(
    masterAccount: string,
    slaveAccount: string,
    settings: import('@/types').SlaveSettings
  ): Promise<void> {
    return this.put(`/trade-groups/${encodeURIComponent(masterAccount)}/members/${encodeURIComponent(slaveAccount)}`, settings);
  }

  /**
   * Delete a TradeGroup member
   */
  async deleteTradeGroupMember(
    masterAccount: string,
    slaveAccount: string
  ): Promise<void> {
    return this.delete(`/trade-groups/${encodeURIComponent(masterAccount)}/members/${encodeURIComponent(slaveAccount)}`);
  }

  /**
   * Toggle a TradeGroup member's status
   */
  async toggleTradeGroupMemberStatus(
    masterAccount: string,
    slaveAccount: string,
    enabled: boolean
  ): Promise<void> {
    return this.post(`/trade-groups/${encodeURIComponent(masterAccount)}/members/${encodeURIComponent(slaveAccount)}/toggle`, { enabled });
  }
}
