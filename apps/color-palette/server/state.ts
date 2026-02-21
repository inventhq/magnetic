export interface ColorGroup {
  name: string;
  shades: { label: string; hex: string }[];
}

export interface AppState {
  palettes: ColorGroup[];
}

export function initialState(): AppState {
  return {
    palettes: [
      {
        name: "Ocean",
        shades: [
          { label: "50", hex: "#eff6ff" },
          { label: "200", hex: "#bfdbfe" },
          { label: "400", hex: "#60a5fa" },
          { label: "600", hex: "#2563eb" },
          { label: "800", hex: "#1e40af" },
          { label: "950", hex: "#172554" },
        ],
      },
      {
        name: "Forest",
        shades: [
          { label: "50", hex: "#f0fdf4" },
          { label: "200", hex: "#bbf7d0" },
          { label: "400", hex: "#4ade80" },
          { label: "600", hex: "#16a34a" },
          { label: "800", hex: "#166534" },
          { label: "950", hex: "#052e16" },
        ],
      },
      {
        name: "Sunset",
        shades: [
          { label: "50", hex: "#fff7ed" },
          { label: "200", hex: "#fed7aa" },
          { label: "400", hex: "#fb923c" },
          { label: "600", hex: "#ea580c" },
          { label: "800", hex: "#9a3412" },
          { label: "950", hex: "#431407" },
        ],
      },
      {
        name: "Violet",
        shades: [
          { label: "50", hex: "#f5f3ff" },
          { label: "200", hex: "#ddd6fe" },
          { label: "400", hex: "#a78bfa" },
          { label: "600", hex: "#7c3aed" },
          { label: "800", hex: "#5b21b6" },
          { label: "950", hex: "#2e1065" },
        ],
      },
      {
        name: "Rose",
        shades: [
          { label: "50", hex: "#fff1f2" },
          { label: "200", hex: "#fecdd3" },
          { label: "400", hex: "#fb7185" },
          { label: "600", hex: "#e11d48" },
          { label: "800", hex: "#9f1239" },
          { label: "950", hex: "#4c0519" },
        ],
      },
    ],
  };
}

export function reduce(state: AppState, _action: string, _payload: any): AppState {
  return state;
}

export function toViewModel(state: AppState) {
  return {
    palettes: state.palettes,
    totalColors: state.palettes.reduce((sum, p) => sum + p.shades.length, 0),
  };
}
