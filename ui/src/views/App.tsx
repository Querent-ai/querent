

import TopBar from '../components/TopBar';
import { CssBaseline, ThemeProvider } from '@mui/material';
import SideBar from '../components/SideBar';
import { Navigate, Route, Routes } from 'react-router-dom';
import IndexesView from './IndexesView';
import { theme } from '../utils/theme';
import IndexView from './IndexView';
import { FullBoxContainer } from '../components/LayoutUtils';
import { LocalStorageProvider } from '../providers/LocalStorageProvider';
import ClusterView from './ClusterView';
import NodeInfoView from './NodeInfoView';
import ApiView from './ApiView';

function App() {
  return (
    <ThemeProvider theme={theme}>
      <LocalStorageProvider>
        <FullBoxContainer sx={{flexDirection: 'row', p: 0}}>
          <CssBaseline />
          <TopBar />
          <SideBar />
          <Routes>
            <Route path="/" element={<Navigate to="/workflows" />} />
            <Route path="workflows" element={<IndexesView />} />
            <Route path="workflows/:pipelineId" element={<IndexView />} />
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
