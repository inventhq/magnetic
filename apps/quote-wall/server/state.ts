export interface Quote {
  text: string;
  author: string;
  category: string;
}

export interface AppState {
  quotes: Quote[];
}

const QUOTES: Quote[] = [
  { text: "The only way to do great work is to love what you do.", author: "Steve Jobs", category: "work" },
  { text: "Innovation distinguishes between a leader and a follower.", author: "Steve Jobs", category: "innovation" },
  { text: "Stay hungry, stay foolish.", author: "Steve Jobs", category: "life" },
  { text: "The best time to plant a tree was 20 years ago. The second best time is now.", author: "Chinese Proverb", category: "wisdom" },
  { text: "Simplicity is the ultimate sophistication.", author: "Leonardo da Vinci", category: "design" },
  { text: "Talk is cheap. Show me the code.", author: "Linus Torvalds", category: "code" },
  { text: "First, solve the problem. Then, write the code.", author: "John Johnson", category: "code" },
  { text: "Code is like humor. When you have to explain it, it's bad.", author: "Cory House", category: "code" },
  { text: "Make it work, make it right, make it fast.", author: "Kent Beck", category: "code" },
  { text: "Perfection is achieved not when there is nothing more to add, but when there is nothing left to take away.", author: "Antoine de Saint-Exupery", category: "design" },
  { text: "The most dangerous phrase in the language is: We've always done it this way.", author: "Grace Hopper", category: "innovation" },
  { text: "Any fool can write code that a computer can understand. Good programmers write code that humans can understand.", author: "Martin Fowler", category: "code" },
];

export function initialState(): AppState {
  return { quotes: QUOTES };
}

export function reduce(state: AppState, _action: string, _payload: any): AppState {
  return state;
}

export function toViewModel(state: AppState) {
  const categories = [...new Set(state.quotes.map(q => q.category))];
  return {
    quotes: state.quotes,
    categories,
    total: state.quotes.length,
  };
}
