export function Filters({
  allClass,
  activeClass,
  doneClass,
}: {
  allClass: string;
  activeClass: string;
  doneClass: string;
}) {
  return (
    <div class="filters" key="filters">
      <button onClick="filter_all" class={`filter-btn ${allClass}`} key="f-all">All</button>
      <button onClick="filter_active" class={`filter-btn ${activeClass}`} key="f-active">Active</button>
      <button onClick="filter_done" class={`filter-btn ${doneClass}`} key="f-done">Done</button>
    </div>
  );
}
