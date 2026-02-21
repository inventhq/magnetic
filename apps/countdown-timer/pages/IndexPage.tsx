export function IndexPage(props: any) {
  const { targetLabel, days, hours, minutes, seconds, isComplete } = props;

  return (
    <div class="min-h-screen bg-surface stack items-center justify-center gap-2xl p-xl">
      <h1 class="text-3xl bold fg-heading text-center">
        {isComplete ? "🎉 Happy " + targetLabel + "!" : "Countdown to " + targetLabel}
      </h1>

      {!isComplete && (
        <div class="row gap-lg justify-center">
          <TimeUnit value={days} label="Days" />
          <TimeUnit value={hours} label="Hours" />
          <TimeUnit value={minutes} label="Minutes" />
          <TimeUnit value={seconds} label="Seconds" />
        </div>
      )}

      <p class="fg-muted text-sm text-center">
        Server-rendered countdown — refreshes on each page load
      </p>
    </div>
  );
}

function TimeUnit({ value, label }: { value: string; label: string }) {
  return (
    <div class="stack items-center gap-sm">
      <div class="bg-raised round-lg p-lg shadow-md" style="min-width: 5rem;">
        <span class="text-3xl bold fg-primary font-mono text-center block">{value}</span>
      </div>
      <span class="text-xs fg-muted uppercase tracking-wide">{label}</span>
    </div>
  );
}
