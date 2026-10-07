import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { createPlannerClient } from '../runtime/tauriClient';
import { PlannerProvider } from '../runtime/PlannerProvider';
import { DevelopmentMealBoard } from '../features/week/DevelopmentMealBoard';
import '../styles/index.css';
const client=createPlannerClient(invoke);
createRoot(document.getElementById('root')!).render(<PlannerProvider client={client}><DevelopmentMealBoard/></PlannerProvider>);
