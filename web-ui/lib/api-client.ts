/**
 * API Client for communicating with Rust Server
 *
 * This client makes direct requests to the Rust server based on the selected Site.
 * It bypasses the Next.js API proxy to support multi-site selection per browser.
 */

import type { Site } from './types/site';

/**
 * API Client class
 */
export class ApiClient {
  private baseUrl: string;

  constructor(site: Site) {
    this.baseUrl = site.siteUrl;
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

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    return response.json();
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

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    return response.json();
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

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    return response.json();
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

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    return response.json();
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

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}: ${response.statusText}`);
    }

    return response.json();
  }
}
