import type { AppProps } from './types.ts';
import { TaskCard } from './TaskCard.tsx';
import { TaskInput } from './TaskInput.tsx';
import { Filters } from './Filters.tsx';

export function App(props: AppProps) {
  return (
    <div class="task-board" key="board">
      <div class="header" key="header">
        <h1 key="title">Task Board</h1>
        <p class="subtitle" key="subtitle">{props.taskCount}</p>
      </div>

      <TaskInput />

      <Filters
        allClass={props.filterAllClass}
        activeClass={props.filterActiveClass}
        doneClass={props.filterDoneClass}
      />

      <div class="task-list" key="task-list">
        {props.visibleTasks.map(task => (
          <TaskCard task={task} />
        ))}
      </div>

      {props.isEmpty && <p class="empty" key="empty">{props.emptyMessage}</p>}
    </div>
  );
}
