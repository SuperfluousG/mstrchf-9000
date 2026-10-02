import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { createPlannerClient } from '../runtime/tauriClient';
import { PlannerProvider } from '../runtime/PlannerProvider';
import { StateWorkbench } from '../features/development/StateWorkbench';
import '../styles/index.css';
const client=createPlannerClient(invoke);
createRoot(document.getElementById('root')!).render(<PlannerProvider client={client}><StateWorkbench/></PlannerProvider>);
