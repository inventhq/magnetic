export function IndexPage({ state }: { state: any }) {
  const { palettes, totalColors } = state;

  return (
    <div class="min-h-screen bg-surface p-xl">
      <div class="stack gap-2xl max-w-lg mx-auto">
        <header class="stack gap-sm text-center">
          <h1 class="text-3xl bold fg-heading">Color Palette</h1>
          <p class="fg-muted text-sm">{totalColors} colors across {palettes.length} palettes</p>
        </header>

        <div class="stack gap-xl">
          {palettes.map((palette: any) => (
            <PaletteGroup palette={palette} />
          ))}
        </div>

        <footer class="text-center fg-muted text-xs p-lg">
          Built with Magnetic
        </footer>
      </div>
    </div>
  );
}

function PaletteGroup({ palette }: { palette: any }) {
  return (
    <div class="stack gap-sm">
      <h2 class="text-lg semibold fg-heading">{palette.name}</h2>
      <div class="row gap-xs wrap">
        {palette.shades.map((shade: any) => (
          <Swatch shade={shade} />
        ))}
      </div>
    </div>
  );
}

function Swatch({ shade }: { shade: any }) {
  return (
    <div class="stack items-center gap-xs" style="flex: 1; min-width: 4.5rem;">
      <div
        class="w-full round-md shadow-sm"
        style={"background:" + shade.hex + ";height:3.5rem;"}
      />
      <span class="text-xs fg-subtle font-mono">{shade.hex}</span>
      <span class="text-xs fg-muted">{shade.label}</span>
    </div>
  );
}
