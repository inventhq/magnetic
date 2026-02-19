import type { TaskView } from './types.ts';

export function TaskCard({ task }: { task: TaskView }) {
  return (
    <div key={`task-${task.id}`} class={`row items-center gap-sm bg-sunken border round-md px-md py-sm transition task-card ${task.cardClass}`}>
      <button onClick={`toggle_${task.id}`} class={`check center shrink-0 fg-muted text-sm cursor-pointer transition ${task.checkClass}`} key={`chk-${task.id}`}>
        {task.checkmark}
      </button>
      <span class={`grow text-base ${task.titleClass}`} key={`tt-${task.id}`}>{task.title}</span>
      <button onClick={`delete_${task.id}`} class="del fg-muted text-lg cursor-pointer transition-colors p-xs" key={`del-${task.id}`}>×</button>
    </div>
  );
}
