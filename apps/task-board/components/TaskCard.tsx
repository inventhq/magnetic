import type { TaskView } from './types.ts';

export function TaskCard({ task }: { task: TaskView }) {
  return (
    <div key={`task-${task.id}`} class={`task-card ${task.completedClass}`}>
      <button onClick={`toggle_${task.id}`} class="check" key={`chk-${task.id}`}>
        {task.checkmark}
      </button>
      <span class="task-title" key={`tt-${task.id}`}>{task.title}</span>
      <button onClick={`delete_${task.id}`} class="delete" key={`del-${task.id}`}>×</button>
    </div>
  );
}
