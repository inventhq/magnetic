// TasksPage — main task board page (/)

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';
import type { AppProps } from '../components/types.ts';
import { TaskCard } from '../components/TaskCard.tsx';
import { TaskInput } from '../components/TaskInput.tsx';
import { Filters } from '../components/Filters.tsx';

export function TasksPage(props: { params: Record<string, string> } & AppProps) {
  return (
    <div class="task-board" key="board">
      <Head>
        <title>{`Tasks (${props.taskCount}) | Magnetic Task Board`}</title>
        <meta name="description" content="A server-driven task board built with Magnetic TSX components" />
        <meta property="og:title" content="Magnetic Task Board" />
      </Head>

      <nav class="topnav" key="nav">
        <Link href="/" class="nav-link active">Tasks</Link>
        <Link href="/about" class="nav-link">About</Link>
      </nav>

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
