import { create } from 'zustand';

interface QueryResult {
  id: string;
  score: number;
  title: string;
  date: string;
  author: string;
  sourceLink: string;
}

interface Query {
  id: string;
  collectionId: string;
  query: string;
  timestamp: number;
  results: QueryResult[];
  timeTaken: number;
}

interface QueryStore {
  queries: Query[];
  currentQueryIndex: number;
  addQuery: (collectionId: string, query: string, results: QueryResult[], timeTaken: number) => void;
  setCurrentQueryIndex: (index: number) => void;
  getCurrentQuery: () => Query | null;
  getPreviousQuery: () => Query | null;
  getNextQuery: () => Query | null;
  hasPreviousQuery: () => boolean;
  hasNextQuery: () => boolean;
  resetQueries: () => void;
}

export const useQueryStore = create<QueryStore>((set, get) => ({
  queries: [],
  currentQueryIndex: -1,

  addQuery: (collectionId: string, query: string, results: QueryResult[], timeTaken: number) => {
    const newQuery: Query = {
      id: Date.now().toString(),
      collectionId,
      query,
      timestamp: Date.now(),
      results,
      timeTaken,
    };

    set((state) => ({
      queries: [...state.queries, newQuery],
      currentQueryIndex: state.queries.length,
    }));
  },

  setCurrentQueryIndex: (index: number) => {
    set({ currentQueryIndex: index });
  },

  getCurrentQuery: () => {
    const { queries, currentQueryIndex } = get();
    return currentQueryIndex >= 0 ? queries[currentQueryIndex] : null;
  },

  getPreviousQuery: () => {
    const { queries, currentQueryIndex } = get();
    return currentQueryIndex > 0 ? queries[currentQueryIndex - 1] : null;
  },

  getNextQuery: () => {
    const { queries, currentQueryIndex } = get();
    return currentQueryIndex < queries.length - 1 ? queries[currentQueryIndex + 1] : null;
  },

  hasPreviousQuery: () => {
    const { currentQueryIndex } = get();
    return currentQueryIndex > 0;
  },

  hasNextQuery: () => {
    const { queries, currentQueryIndex } = get();
    return currentQueryIndex < queries.length - 1;
  },

  resetQueries: () => {
    set({ queries: [], currentQueryIndex: -1 });
  },
})); 