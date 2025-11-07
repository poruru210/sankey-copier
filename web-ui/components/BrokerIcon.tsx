'use client';

import { useState } from 'react';
import { Building2 } from 'lucide-react';
import { getBrokerIconUrl, getBrokerColor } from '@/lib/brokerIcons';

interface BrokerIconProps {
  brokerName: string;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

/**
 * Broker icon component with automatic favicon fetching and fallback
 */
export function BrokerIcon({ brokerName, size = 'md', className = '' }: BrokerIconProps) {
  const [imageError, setImageError] = useState(false);
  const iconUrl = getBrokerIconUrl(brokerName);

  // Size mappings
  const sizeClasses = {
    sm: 'w-6 h-6',
    md: 'w-7 h-7',
    lg: 'w-8 h-8',
  };

  const iconSizeClasses = {
    sm: 'w-3 h-3',
    md: 'w-4 h-4',
    lg: 'w-5 h-5',
  };

  // If no icon URL or image failed to load, show fallback
  if (!iconUrl || imageError) {
    const colorClass = getBrokerColor(brokerName);
    return (
      <div
        className={`${sizeClasses[size]} ${colorClass} rounded flex items-center justify-center flex-shrink-0 ${className}`}
      >
        <Building2 className={`${iconSizeClasses[size]} text-white`} />
      </div>
    );
  }

  // Show broker favicon
  return (
    <div
      className={`${sizeClasses[size]} rounded flex items-center justify-center flex-shrink-0 ${className}`}
    >
      <img
        src={iconUrl}
        alt={`${brokerName} icon`}
        className="w-full h-full object-contain"
        onError={() => setImageError(true)}
      />
    </div>
  );
}
