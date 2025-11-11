import { NextRequest, NextResponse } from 'next/server';

/**
 * Dynamic API proxy route for all /api/* requests
 *
 * This proxies all API requests to the Rust Server backend.
 * The target URL is determined at runtime from the NEXT_PUBLIC_API_URL environment variable,
 * allowing the installer to configure the Rust Server port dynamically.
 *
 * This is necessary because Next.js rewrites in next.config.ts are evaluated at build time,
 * not runtime, so they cannot use environment variables set by NSSM during installation.
 */

// Get Rust Server API URL from environment variable
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

/**
 * GET request handler
 */
export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRequest(request, path, 'GET');
}

/**
 * POST request handler
 */
export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRequest(request, path, 'POST');
}

/**
 * PUT request handler
 */
export async function PUT(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRequest(request, path, 'PUT');
}

/**
 * DELETE request handler
 */
export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRequest(request, path, 'DELETE');
}

/**
 * PATCH request handler
 */
export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRequest(request, path, 'PATCH');
}

/**
 * Proxy a request to the Rust Server
 */
async function proxyRequest(
  request: NextRequest,
  path: string[],
  method: string
): Promise<NextResponse> {
  try {
    const pathString = path.join('/');
    const searchParams = request.nextUrl.searchParams.toString();
    const targetUrl = API_BASE_URL + '/api/' + pathString + (searchParams ? '?' + searchParams : '');

    const headers = new Headers(request.headers);
    headers.delete('host');
    headers.delete('connection');

    const body = method !== 'GET' && method !== 'HEAD'
      ? await request.text()
      : undefined;

    const response = await fetch(targetUrl, {
      method,
      headers,
      body,
    });

    const responseBody = await response.text();

    const proxyResponse = new NextResponse(responseBody, {
      status: response.status,
      statusText: response.statusText,
    });

    response.headers.forEach((value, key) => {
      proxyResponse.headers.set(key, value);
    });

    return proxyResponse;
  } catch (error) {
    console.error('Failed to proxy request:', error);
    return NextResponse.json(
      {
        error: 'Failed to connect to Rust Server',
        details: error instanceof Error ? error.message : 'Unknown error'
      },
      { status: 502 }
    );
  }
}
