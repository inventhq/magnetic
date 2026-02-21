// state.ts — Shipment tracker state
// All data comes from the SSE data source, so state is minimal.

export interface AppState {
  connected: boolean;
}

export function initialState(): AppState {
  return {
    connected: true,
  };
}

export function reduce(state: AppState, action: string, _payload: any): AppState {
  return state;
}

export function toViewModel(state: any) {
  // The data layer merges SSE events into state as `events` (array of last 20)
  const events = state.events || [];
  const count = events.length;

  return {
    events,
    count,
    hasEvents: count > 0,
    statusText: count > 0 ? `${count} events received` : 'Waiting for events...',
  };
}
