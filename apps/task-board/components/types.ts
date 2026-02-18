// Shared types for task-board components

export interface Task {
  id: number;
  title: string;
  completed: boolean;
}

export interface TaskView extends Task {
  completedClass: string;
  checkmark: string;
}

export interface AppProps {
  taskCount: string;
  visibleTasks: TaskView[];
  filterAllClass: string;
  filterActiveClass: string;
  filterDoneClass: string;
  isEmpty: boolean;
  emptyMessage: string;
}
