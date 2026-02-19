// UsersPage — displays users fetched from JSONPlaceholder API (page-scoped)
export function UsersPage(props: any) {
  const users = props.users || [];
  return (
    <div key="app">
      <h1 key="title">Users</h1>
      <p key="subtitle">Page-scoped data: only fetched when navigating to /users</p>
      <p key="count">{users.length} users loaded</p>
      <ul key="users">
        {users.map((user: any) => (
          <li key={`user-${user.id}`}>
            <strong>{user.name}</strong> — {user.email}
          </li>
        ))}
      </ul>
      <hr key="sep" />
      <p key="nav">
        <a href="/" data-a_click="navigate" data-payload={JSON.stringify({ path: "/" })}>
          ← Back to Posts
        </a>
      </p>
    </div>
  );
}
