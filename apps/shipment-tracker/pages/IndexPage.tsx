// IndexPage — Live shipment event feed from SSE stream

import { Head } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';

function carrierBadgeClass(carrier: string): string {
  const c = (carrier || '').toLowerCase();
  if (c === 'ups') return 'bg-warning fg-surface';
  if (c === 'fedex') return 'bg-info fg-surface';
  if (c === 'usps') return 'bg-primary fg-heading';
  return 'bg-raised fg-muted';
}

function eventTypeBadgeClass(eventType: string): string {
  if (eventType.includes('delivered')) return 'bg-success fg-surface';
  if (eventType.includes('transit') || eventType.includes('pickup')) return 'bg-info fg-surface';
  if (eventType.includes('exception') || eventType.includes('alert')) return 'bg-danger fg-heading';
  return 'bg-raised fg-text';
}

function formatTime(tsMs: number): string {
  if (!tsMs) return '';
  const d = new Date(tsMs);
  const h = d.getHours().toString().padStart(2, '0');
  const m = d.getMinutes().toString().padStart(2, '0');
  const s = d.getSeconds().toString().padStart(2, '0');
  return `${h}:${m}:${s}`;
}

function EventCard({ event }: { event: any }) {
  const params = event.params || {};
  const eventType = event.event_type || 'unknown';
  const carrier = params.carrier || '';
  const tracking = params.tracking_number || '';
  const time = formatTime(event.timestamp_ms);

  return (
    <div class="row gap-md items-center p-md bg-raised border round-md" key={event.event_id}>
      <div class="stack gap-xs grow" key="info">
        <div class="row gap-sm items-center" key="badges">
          <span class={`text-xs bold px-sm py-xs round-sm ${eventTypeBadgeClass(eventType)}`} key="type">
            {eventType}
          </span>
          {carrier && (
            <span class={`text-xs bold px-sm py-xs round-sm uppercase ${carrierBadgeClass(carrier)}`} key="carrier">
              {carrier}
            </span>
          )}
          <span class="text-xs fg-muted" key="time">{time}</span>
        </div>
        {tracking && (
          <span class="text-sm font-mono fg-subtle" key="tracking">{tracking}</span>
        )}
      </div>
      <span class="text-xs fg-muted font-mono truncate" key="id" style="max-width:120px">
        {(event.event_id || '').slice(0, 12)}
      </span>
    </div>
  );
}

export function IndexPage(props: any) {
  const events = props.events || [];
  const hasEvents = events.length > 0;

  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <Head>
        <title>Live Shipment Events | Magnetic</title>
        <meta name="description" content="Real-time shipment event feed powered by Magnetic SSE" />
      </Head>

      <div class="stack gap-md w-full max-w-lg" key="main">
        <div class="stack gap-xs text-center" key="header">
          <h1 class="text-2xl bold fg-heading" key="title">Live Shipment Events</h1>
          <p class="text-sm fg-muted" key="status">{props.statusText}</p>
        </div>

        <div class="row gap-sm items-center justify-center" key="indicator">
          <span class="inline-block round-full bg-success" style="width:8px;height:8px" key="dot"></span>
          <span class="text-xs fg-subtle" key="label">Connected to SSE stream</span>
        </div>

        <div class="stack gap-sm" key="feed">
          {hasEvents
            ? events.slice().reverse().map((ev: any) => <EventCard event={ev} />)
            : (
              <div class="text-center p-2xl fg-muted" key="empty">
                <p class="text-lg" key="wait">Waiting for events...</p>
                <p class="text-sm" key="hint">Events will appear here in real-time</p>
              </div>
            )
          }
        </div>
      </div>
    </div>
  );
}
