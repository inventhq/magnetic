// TasksPage — main task board page (/)

import { Head, Link } from '../../../js/packages/magnetic-server/src/jsx-runtime.ts';
import type { AppProps } from '../components/types.ts';
import { TaskCard } from '../components/TaskCard.tsx';
import { TaskInput } from '../components/TaskInput.tsx';
import { Filters } from '../components/Filters.tsx';

export function TasksPage(props: { params: Record<string, string> } & AppProps) {
  return (
    <div class="stack items-center p-xl min-h-screen" key="wrapper">
      <div class="stack gap-md w-full bg-raised border round-lg p-lg shadow-lg board" key="board">
        <Head>
          <title>{`Tasks (${props.taskCount}) | Magnetic Task Board`}</title>
          <meta name="description" content="A server-driven task board built with Magnetic TSX components" />
          <meta property="og:title" content="Magnetic Task Board" />
        </Head>

        <nav class="row gap-md justify-center" key="nav">
          <Link href="/" class="nav-link text-sm medium fg-primary" prefetch>Tasks</Link>
          <Link href="/about" class="nav-link text-sm medium fg-muted transition-colors" prefetch>About</Link>
        </nav>

        <div class="text-center" key="header">
          <h1 class="text-3xl bold fg-heading" key="title">Task Board</h1>
          <p class="text-sm fg-muted" key="subtitle">{props.taskCount}</p>
        </div>

        <TaskInput />

        <Filters
          allClass={props.filterAllClass}
          activeClass={props.filterActiveClass}
          doneClass={props.filterDoneClass}
        />

        <div class="stack gap-sm" key="task-list">
          {props.visibleTasks.map(task => (
            <TaskCard task={task} />
          ))}
        </div>

        {props.isEmpty && <p class="text-center fg-muted text-sm italic py-xl" key="empty">{props.emptyMessage}</p>}
      </div>
    </div>
  );
}
