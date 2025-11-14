/**
 * Broker icon configuration and utilities
 * Maps broker names to their website domains for favicon fetching
 */

// Map broker names (normalized) to their website domains
const BROKER_DOMAINS: Record<string, string> = {
  'exness': 'exness.com',
  'xm': 'xm.com',
  'fxpro': 'fxpro.com',
  'ic markets': 'icmarkets.com',
  'pepperstone': 'pepperstone.com',
  'oanda': 'oanda.com',
  'fxcm': 'fxcm.com',
  'ig': 'ig.com',
  'avatrade': 'avatrade.com',
  'etoro': 'etoro.com',
  'plus500': 'plus500.com',
  'tradexfin': 'xm.com', // Tradexfin Limited is XM's legal entity
  'fxtm': 'fxtm.com',
  'hotforex': 'hfm.com',
  'roboforex': 'roboforex.com',
  'octafx': 'octafx.com',
  'forex.com': 'forex.com',
  'tickmill': 'tickmill.com',
  'fbs': 'fbs.com',
};

/**
 * Get broker favicon URL using Google S2 API
 * Falls back to a colored icon if broker domain is not found
 */
export function getBrokerIconUrl(brokerName: string): string {
  // Normalize broker name (lowercase, remove special chars)
  const normalized = brokerName.toLowerCase().replace(/[^a-z0-9\s]/g, '').trim();

  // Try exact match first
  let domain = BROKER_DOMAINS[normalized];

  // If no exact match, try partial match (check if any key is contained in the broker name)
  if (!domain) {
    for (const [key, value] of Object.entries(BROKER_DOMAINS)) {
      if (normalized.includes(key) || key.includes(normalized)) {
        domain = value;
        break;
      }
    }
  }

  if (domain) {
    // Use Google S2 API to fetch favicon
    // Size options: 16, 32, 64, 128, 256
    return `https://www.google.com/s2/favicons?domain=${domain}&sz=64`;
  }

  // Return empty string to use fallback icon
  return '';
}

/**
 * Get a deterministic color for a broker based on its name
 * Used as fallback when favicon is not available
 */
export function getBrokerColor(brokerName: string): string {
  const colors = [
    'bg-blue-500',
    'bg-green-500',
    'bg-purple-500',
    'bg-pink-500',
    'bg-indigo-500',
    'bg-yellow-500',
    'bg-red-500',
    'bg-cyan-500',
    'bg-orange-500',
    'bg-teal-500',
  ];

  // Generate a hash from the broker name
  let hash = 0;
  for (let i = 0; i < brokerName.length; i++) {
    hash = ((hash << 5) - hash) + brokerName.charCodeAt(i);
    hash = hash & hash; // Convert to 32-bit integer
  }

  // Use hash to select a color
  const index = Math.abs(hash) % colors.length;
  return colors[index];
}

/**
 * Extract broker name from account name format
 * Format: "Broker_Name_AccountNumber" or "Broker Name"
 */
export function extractBrokerName(accountName: string): string {
  const lastUnderscoreIndex = accountName.lastIndexOf('_');
  if (lastUnderscoreIndex === -1) {
    return accountName;
  }
  return accountName.substring(0, lastUnderscoreIndex).replace(/_/g, ' ');
}
