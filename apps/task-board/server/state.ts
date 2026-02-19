// state.ts — App state and reducers for task board

export interface Task {
  id: number;
  title: string;
  completed: boolean;
}

export interface AppState {
  tasks: Task[];
  filter: 'all' | 'active' | 'done';
  nextId: number;
}

export function initialState(): AppState {
  return {
    tasks: [
      { id: 1, title: 'Build magnetic components', completed: true },
      { id: 2, title: 'Wire up WASM transport layer', completed: true },
      { id: 3, title: 'Implement POST response architecture', completed: false },
      { id: 4, title: 'Add routing support', completed: false },
      { id: 5, title: 'Deploy to edge', completed: false },
    ],
    filter: 'all',
    nextId: 6,
  };
}

export function reduce(state: AppState, action: string, payload: any): AppState {
  switch (action) {
    case 'add_task': {
      const title = payload?.title?.trim();
      if (!title) return state;
      return {
        ...state,
        tasks: [...state.tasks, { id: state.nextId, title, completed: false }],
        nextId: state.nextId + 1,
      };
    }

    case 'delete_task': {
      const id = parseInt(action.split('_').pop() || payload?.id, 10);
      return {
        ...state,
        tasks: state.tasks.filter(t => t.id !== id),
      };
    }

    case 'filter_all':
      return { ...state, filter: 'all' };
    case 'filter_active':
      return { ...state, filter: 'active' };
    case 'filter_done':
      return { ...state, filter: 'done' };

    default: {
      // Handle parameterized actions: toggle_42, delete_42
      const toggleMatch = action.match(/^toggle_(\d+)$/);
      if (toggleMatch) {
        const id = parseInt(toggleMatch[1], 10);
        return {
          ...state,
          tasks: state.tasks.map(t =>
            t.id === id ? { ...t, completed: !t.completed } : t
          ),
        };
      }

      const deleteMatch = action.match(/^delete_(\d+)$/);
      if (deleteMatch) {
        const id = parseInt(deleteMatch[1], 10);
        return {
          ...state,
          tasks: state.tasks.filter(t => t.id !== id),
        };
      }

      return state;
    }
  }
}

// Transform raw state into view model for components
export function toViewModel(state: AppState) {
  const visibleTasks = state.tasks
    .filter(t => {
      if (state.filter === 'active') return !t.completed;
      if (state.filter === 'done') return t.completed;
      return true;
    })
    .map(t => ({
      ...t,
      cardClass: t.completed ? 'opacity-50' : '',
      titleClass: t.completed ? 'line-through fg-muted' : '',
      checkClass: t.completed ? 'check-done' : '',
      checkmark: t.completed ? '✓' : '○',
    }));

  const active = state.tasks.filter(t => !t.completed).length;
  const done = state.tasks.filter(t => t.completed).length;

  return {
    ...state,
    visibleTasks,
    taskCount: `${active} active, ${done} done`,
    filterAllClass: state.filter === 'all' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    filterActiveClass: state.filter === 'active' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    filterDoneClass: state.filter === 'done' ? 'bg-primary fg-heading border-primary' : 'bg-raised fg-muted',
    isEmpty: visibleTasks.length === 0,
    emptyMessage:
      state.filter === 'active'
        ? 'All tasks completed!'
        : state.filter === 'done'
          ? 'No completed tasks yet'
          : 'No tasks yet. Add one above!',
  };
}
