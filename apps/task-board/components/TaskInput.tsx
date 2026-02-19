export function TaskInput() {
  return (
    <form class="row gap-sm" onSubmit="add_task" key="add-form">
      <input type="text" name="title" placeholder="Add a task..." autocomplete="off"
        class="add-input grow bg-sunken border round-md px-md py-sm fg-text text-base transition" key="add-input" />
      <button type="submit"
        class="add-btn bg-primary fg-heading round-md px-lg py-sm text-base semibold cursor-pointer transition" key="add-btn">Add</button>
    </form>
  );
}
