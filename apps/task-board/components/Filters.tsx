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
    <div class="row gap-xs justify-center" key="filters">
      <button onClick="filter_all" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${allClass}`} key="f-all">All</button>
      <button onClick="filter_active" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${activeClass}`} key="f-active">Active</button>
      <button onClick="filter_done" class={`filter-btn border round-sm px-sm py-xs text-sm cursor-pointer transition ${doneClass}`} key="f-done">Done</button>
    </div>
  );
}
