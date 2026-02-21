export function IndexPage(props: any) {
  const { quotes, total } = props;

  return (
    <div class="min-h-screen bg-surface p-xl">
      <div class="stack gap-xl max-w-lg mx-auto">
        <header class="stack gap-sm text-center">
          <h1 class="text-3xl bold fg-heading">Quote Wall</h1>
          <p class="fg-muted text-sm">{total} quotes to inspire your day</p>
        </header>

        <div class="stack gap-md">
          {quotes.map((q: any, i: number) => (
            <QuoteCard key={i} quote={q} />
          ))}
        </div>

        <footer class="text-center fg-muted text-xs p-lg">
          Built with Magnetic
        </footer>
      </div>
    </div>
  );
}

function QuoteCard({ quote }: { quote: any }) {
  const categoryColors: Record<string, string> = {
    code: "fg-primary",
    design: "fg-success",
    innovation: "fg-warning",
    work: "fg-info",
    life: "fg-danger",
    wisdom: "fg-subtle",
  };
  const colorClass = categoryColors[quote.category] || "fg-muted";

  return (
    <div class="bg-raised round-md p-lg border border-border stack gap-sm">
      <p class="fg-text text-base leading-relaxed italic">"{quote.text}"</p>
      <div class="row justify-between items-center">
        <span class="fg-subtle text-sm">— {quote.author}</span>
        <span class={"text-xs uppercase tracking-wide " + colorClass}>{quote.category}</span>
      </div>
    </div>
  );
}
