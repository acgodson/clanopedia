// src/clanopedia_frontend/src/hooks/useAuth.js

import { useState, useEffect, createContext, useContext } from 'react';
import { AuthClient } from '@dfinity/auth-client';
import { Actor, HttpAgent } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { idlFactory as clanopediaIdlFactory } from 'declarations/clanopedia_backend/clanopedia_backend.did.js';
import { idlFactory as bluebandIdlFactory } from 'declarations/blueband_rust/blueband_rust.did.js';
import type { _SERVICE as ClanopediaService } from 'declarations/clanopedia_backend/clanopedia_backend.did.d.ts';
import type { _SERVICE as BluebandService } from 'declarations/blueband_rust/blueband_rust.did.d.ts';

interface AuthContextType {
  isAuthenticated: boolean;
  principal: Principal | null;
  login: () => Promise<void>;
  logout: () => Promise<void>;
  authClient: AuthClient | null;
  ClanopediaActor: ClanopediaService | null;
  BluebandActor: BluebandService | null;
  isLoading: boolean;
  principalText: string | null;
}

const AuthContext = createContext<AuthContextType | null>(null);

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};

export const AuthProvider = ({ children }: { children: React.ReactNode }) => {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [principal, setPrincipal] = useState<Principal | null>(null);
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [ClanopediaActor, setClanopediaActor] = useState<ClanopediaService | null>(null);
  const [BluebandActor, setBluebandActor] = useState<BluebandService | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Create authenticated actor helper function
  const createAuthenticatedActor = async (identity: any) => {
    try {
      const agent = await HttpAgent.create({
        identity,
        shouldFetchRootKey: import.meta.env.DEV
      });

      // Verify the agent is properly configured
      const agentPrincipal = await agent.getPrincipal();
      if (!agentPrincipal.isAnonymous()) {
        // Create Clanopedia actor
        const clanopediaActor = Actor.createActor<ClanopediaService>(clanopediaIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_CLANOPEDIA_BACKEND,
        });

        // Create Blueband actor
        const bluebandActor = Actor.createActor<BluebandService>(bluebandIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_BLUEBAND_RUST,
        });

        return { clanopediaActor, bluebandActor };
      }
      throw new Error('Agent principal is anonymous');
    } catch (error) {
      console.error('Failed to create authenticated actors:', error);
      throw error;
    }
  };

  useEffect(() => {
    initAuth();
  }, []);

  const initAuth = async () => {
    try {
      const client = await AuthClient.create();
      setAuthClient(client);

      const isAuth = await client.isAuthenticated();
      setIsAuthenticated(isAuth);

      if (isAuth) {
        const identity = client.getIdentity();
        const principalId = identity.getPrincipal();
        setPrincipal(principalId);

        try {
          const { clanopediaActor, bluebandActor } = await createAuthenticatedActor(identity);
          setClanopediaActor(clanopediaActor);
          setBluebandActor(bluebandActor);
        } catch (error) {
          console.error('Failed to initialize authenticated actors:', error);
          // Don't fall back to anonymous actors if we're authenticated
          throw error;
        }
      } else {
        // Create anonymous actors
        const agent = new HttpAgent({ shouldFetchRootKey: import.meta.env.DEV });
        const clanopediaActor = Actor.createActor<ClanopediaService>(clanopediaIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_CLANOPEDIA_BACKEND,
        });
        const bluebandActor = Actor.createActor<BluebandService>(bluebandIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_BLUEBAND_RUST,
        });
        setClanopediaActor(clanopediaActor);
        setBluebandActor(bluebandActor);
      }
    } catch (error) {
      console.error('Auth initialization failed:', error);
      // Only fall back to anonymous actors if we're not authenticated
      if (!isAuthenticated) {
        const agent = new HttpAgent({ shouldFetchRootKey: import.meta.env.DEV });
        const clanopediaActor = Actor.createActor<ClanopediaService>(clanopediaIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_CLANOPEDIA_BACKEND,
        });
        const bluebandActor = Actor.createActor<BluebandService>(bluebandIdlFactory, {
          agent,
          canisterId: import.meta.env.CANISTER_ID_BLUEBAND_RUST,
        });
        setClanopediaActor(clanopediaActor);
        setBluebandActor(bluebandActor);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const login = async () => {
    if (!authClient) return;

    try {
      setIsLoading(true);

      console.log('Environment:', {
        mode: import.meta.env.VITE_MODE,
        isDev: import.meta.env.DEV,
        isProd: import.meta.env.PROD,
        canisterId: import.meta.env.CANISTER_ID_INTERNET_IDENTITY
      });

      await authClient.login({
        identityProvider:
          import.meta.env.DEV
            ? `http://${import.meta.env.CANISTER_ID_INTERNET_IDENTITY}.localhost:4943`
            : "https://identity.ic0.app",
        maxTimeToLive: BigInt(8 * 60 * 60 * 1000 * 1000 * 1000),
        onSuccess: async () => {
          try {
            const identity = authClient.getIdentity();
            const principalId = identity.getPrincipal();

            setIsAuthenticated(true);
            setPrincipal(principalId);

            const { clanopediaActor, bluebandActor } = await createAuthenticatedActor(identity);
            setClanopediaActor(clanopediaActor);
            setBluebandActor(bluebandActor);
          } catch (error) {
            console.error('Failed to create actors after login:', error);
            // Don't fall back to anonymous actors
            throw error;
          }
        },
      });
    } catch (error) {
      console.error('Login failed:', error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };

  const logout = async () => {
    if (!authClient) return;

    try {
      await authClient.logout();
      setIsAuthenticated(false);
      setPrincipal(null);
      // Create anonymous actors
      const agent = new HttpAgent({ shouldFetchRootKey: import.meta.env.DEV });
      const clanopediaActor = Actor.createActor<ClanopediaService>(clanopediaIdlFactory, {
        agent,
        canisterId: import.meta.env.CANISTER_ID_CLANOPEDIA_BACKEND,
      });
      const bluebandActor = Actor.createActor<BluebandService>(bluebandIdlFactory, {
        agent,
        canisterId: import.meta.env.CANISTER_ID_BLUEBAND_RUST,
      });
      setClanopediaActor(clanopediaActor);
      setBluebandActor(bluebandActor);
    } catch (error) {
      console.error('Logout failed:', error);
    }
  };

  const value = {
    isAuthenticated,
    principal,
    login,
    logout,
    authClient,
    ClanopediaActor,
    BluebandActor,
    isLoading,
    principalText: principal ? principal.toString() : null,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};