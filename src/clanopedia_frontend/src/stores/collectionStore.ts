import { create } from 'zustand';
import { Principal } from '@dfinity/principal';
import { Collection } from '../../../declarations/clanopedia_backend/clanopedia_backend.did';

interface CollectionWithMetadata extends Collection {
  isOwner: boolean;
  isAdmin: boolean;
}

interface CollectionStore {
  collections: CollectionWithMetadata[];
  isLoading: boolean;
  error: string | null;
  activeTab: 'all' | 'forYou';
  setActiveTab: (tab: 'all' | 'forYou') => void;
  fetchCollections: (actor: any, principal: Principal | null) => Promise<void>;
}

export const useCollectionStore = create<CollectionStore>((set) => ({
  collections: [],
  isLoading: false,
  error: null,
  activeTab: 'all',
  setActiveTab: (tab) => set({ activeTab: tab }),
  
  fetchCollections: async (actor, principal) => {
    set({ isLoading: true, error: null });
    try {
      const result = await actor.list_collections();
      
      if ('Ok' in result) {
        const collectionsWithMetadata = result.Ok.map((collection: Collection) => ({
          ...collection,
          isOwner: principal ? collection.creator.toString() === principal.toString() : false,
          isAdmin: principal ? collection.admins.some(admin => admin.toString() === principal.toString()) : false,
        }));
        
        set({ collections: collectionsWithMetadata });
      } else {
        set({ error: 'Failed to fetch collections' });
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'An error occurred' });
    } finally {
      set({ isLoading: false });
    }
  },
})); 