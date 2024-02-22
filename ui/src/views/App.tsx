

import TopBar from '../components/TopBar';
import { CssBaseline, ThemeProvider } from '@mui/material';
import SideBar from '../components/SideBar';
import { Navigate, Route, Routes } from 'react-router-dom';
import { theme } from '../utils/theme';
import { FullBoxContainer } from '../components/LayoutUtils';
import { LocalStorageProvider } from '../providers/LocalStorageProvider';
import ClusterView from './ClusterView';
import NodeInfoView from './NodeInfoView';
import ApiView from './ApiView';
import WorkflowsView from './WorkflowsView';

function App() {
  return (
    <ThemeProvider theme={theme}>
      <LocalStorageProvider>
        <FullBoxContainer sx={{flexDirection: 'row', p: 0}}>
          <CssBaseline />
          <TopBar />
          <></>
          <SideBar />
          <Routes>
            <Route path="/" element={<Navigate to="/workflows" />} />
            <Route path="workflows" element={<WorkflowsView />} />
            <Route path="cluster" element={<ClusterView />} />
            <Route path="node-info" element={<NodeInfoView />} />
            <Route path="api-playground" element={<ApiView />} />
          </Routes>
        </FullBoxContainer>
      </LocalStorageProvider>
    </ThemeProvider>
  );
}

export default App;
