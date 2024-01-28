

import { TabContext, TabList, TabPanel } from '@mui/lab';
import { Box, Tab, Typography, styled } from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import ApiUrlFooter from '../components/ApiUrlFooter';
import { JsonEditor } from '../components/JsonEditor';
import { ViewUnderAppBarBox, FullBoxContainer, QBreadcrumbs } from '../components/LayoutUtils';
import Loader from '../components/Loader';
import { Client } from '../services/client';
import { QuesterBuildInfo } from '../utils/models';

const CustomTabPanel = styled(TabPanel)`
padding-left: 0;
padding-right: 0;
height: 100%;
`;

function NodeInfoView() {
  const [loadingCounter, setLoadingCounter] = useState(2);
  const [nodeId, setNodeId] = useState<string>("");
  // eslint-disable-next-line
  const [nodeConfig, setNodeConfig] = useState<null | Record<string, any>>(null);
  const [buildInfo, setBuildInfo] = useState<null | QuesterBuildInfo>(null);
  const [tabIndex, setTabIndex] = useState('1');
  const questerClient = useMemo(() => new Client(), []);

  const urlByTab: Record<string, string> = {
    '1': 'api/v1/config',
    '2': 'api/v1/version',
  }

  const handleTabIndexChange = (_: React.SyntheticEvent, newValue: string) => {
    setTabIndex(newValue);
  };

  useEffect(() => {
    questerClient.cluster().then(
      (cluster) => {
        setNodeId(cluster.node_id);
      },
      (error) => {
        console.log('Error when fetching cluster info:', error);
      }
    )
  });
  useEffect(() => {
    setLoadingCounter(2);
    questerClient.buildInfo().then(
      (fetchedBuildInfo) => {
        setLoadingCounter(prevCounter => prevCounter - 1);
        setBuildInfo(fetchedBuildInfo);
      },
      (error) => {
        setLoadingCounter(prevCounter => prevCounter - 1);
        console.log('Error when fetching build info: ', error);
      }
    );
    questerClient.config().then(
      (fetchedConfig) => {
        setLoadingCounter(prevCounter => prevCounter - 1);
        setNodeConfig(fetchedConfig);
      },
      (error) => {
        setLoadingCounter(prevCounter => prevCounter - 1);
        console.log('Error when fetching node config: ', error);
      }
    );
  }, [questerClient]);

  const renderResult = () => {
    if (loadingCounter !== 0) {
      return <Loader />;
    } else {
      return <FullBoxContainer sx={{ px: 0 }}>
        <TabContext value={tabIndex}>
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <TabList onChange={handleTabIndexChange} aria-label="Node tabs">
              <Tab label="Node config" value="1" />
              <Tab label="Build info" value="2" />
            </TabList>
          </Box>
          <CustomTabPanel value="1">
            <JsonEditor content={nodeConfig} resizeOnMount={false} />
          </CustomTabPanel>
          <CustomTabPanel value="2">
            <JsonEditor content={buildInfo} resizeOnMount={false} />
          </CustomTabPanel>
        </TabContext>
      </FullBoxContainer>
    }
  }

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Node ID: {nodeId} (self)</Typography>
        </QBreadcrumbs>
        { renderResult() }
      </FullBoxContainer>
      { ApiUrlFooter(urlByTab[tabIndex] || '') }
    </ViewUnderAppBarBox>
  );
}

export default NodeInfoView;
