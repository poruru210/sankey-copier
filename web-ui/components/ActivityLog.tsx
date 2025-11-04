'use client';

import { useIntlayer } from 'next-intlayer';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

interface ActivityLogProps {
  messages: string[];
}

export function ActivityLog({ messages }: ActivityLogProps) {
  const { title, noActivity } = useIntlayer('activity-log');

  return (
    <Card className="mb-6 mt-6">
      <CardHeader>
        <CardTitle className="text-xl">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 max-h-40 overflow-y-auto">
          {messages.length === 0 ? (
            <p className="text-muted-foreground text-sm">{noActivity}</p>
          ) : (
            messages.map((msg, idx) => (
              <div key={idx} className="text-sm font-mono bg-muted p-2 rounded">
                {msg}
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
