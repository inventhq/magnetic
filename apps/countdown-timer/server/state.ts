export interface AppState {
  targetLabel: string;
  targetDate: string;
}

export function initialState(): AppState {
  return {
    targetLabel: "New Year 2027",
    targetDate: "2027-01-01T00:00:00Z",
  };
}

export function reduce(state: AppState, _action: string, _payload: any): AppState {
  return state;
}

export function toViewModel(state: AppState) {
  const now = Date.now();
  const target = new Date(state.targetDate).getTime();
  const diff = Math.max(0, target - now);

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff / (1000 * 60 * 60)) % 24);
  const minutes = Math.floor((diff / (1000 * 60)) % 60);
  const seconds = Math.floor((diff / 1000) % 60);

  return {
    targetLabel: state.targetLabel,
    days: String(days).padStart(3, '0'),
    hours: String(hours).padStart(2, '0'),
    minutes: String(minutes).padStart(2, '0'),
    seconds: String(seconds).padStart(2, '0'),
    isComplete: diff === 0,
  };
}
