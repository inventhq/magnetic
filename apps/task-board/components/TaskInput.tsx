export function TaskInput() {
  return (
    <form class="add-form" onSubmit="add_task" key="add-form">
      <input type="text" name="title" placeholder="Add a task..." autocomplete="off" key="add-input" />
      <button type="submit" key="add-btn">Add</button>
    </form>
  );
}
