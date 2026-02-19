// IndexPage — displays posts fetched from JSONPlaceholder API via data layer
export function IndexPage(props: any) {
  const posts = props.posts || [];
  return (
    <div key="app">
      <h1 key="title">Magnetic Data Layer Demo</h1>
      <p key="subtitle">Posts from JSONPlaceholder (fetched server-side, zero client JS)</p>
      <p key="count">{posts.length} posts loaded</p>
      <ul key="posts">
        {posts.map((post: any) => (
          <li key={`post-${post.id}`}>
            <strong>{post.title}</strong>
            <p>{post.body?.substring(0, 80)}...</p>
          </li>
        ))}
      </ul>
      <hr key="sep" />
      <p key="nav">
        <a href="/users" data-a_click="navigate" data-payload={JSON.stringify({ path: "/users" })}>
          View Users →
        </a>
      </p>
    </div>
  );
}
