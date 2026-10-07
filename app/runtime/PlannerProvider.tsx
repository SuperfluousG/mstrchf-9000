import { createContext, useContext, useEffect, useMemo, useSyncExternalStore, type ReactNode } from 'react';
import type { PlannerClient } from './PlannerClient';
import { PlannerSession } from './commandRecovery';
const Context = createContext<{session:PlannerSession;client:PlannerClient}|null>(null);
export function PlannerProvider({client,children}:{client:PlannerClient;children:ReactNode}) {
  const value=useMemo(()=>({client,session:new PlannerSession(client)}),[client]);
  useEffect(()=>{ void value.session.refresh(); },[value]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}
export function usePlanner() {
  const value=useContext(Context);
  if(!value) throw new Error('PlannerProvider is required');
  const state=useSyncExternalStore(value.session.subscribe,value.session.getState);
  return {...value,state};
}
