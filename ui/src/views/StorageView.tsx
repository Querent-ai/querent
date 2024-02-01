import { useEffect, useState } from 'react';
import { styled, Card, CardContent, Typography, Box } from '@mui/material';
import { Client } from '../services/client';
import Loader from '../components/Loader';
import { NodeConfig } from '../utils/models';

const StyledCard = styled(Card)(({ theme }) => ({
  backgroundColor: theme.palette.primary.main,
  color: theme.palette.common.white,
  marginBottom: theme.spacing(2),
  width: '200px', // Adjust the width as needed
}));

function StorageView() {
  const [nodeConfig, setNodeConfig] = useState<null | NodeConfig>(null);
  const questerClient = new Client();

  useEffect(() => {
    const fetchNodeConfig = async () => {
      try {
        const config = await questerClient.node_config();
        setNodeConfig(config);
      } catch (error) {
        console.error('Error fetching node config:', error);
      }
    };

    fetchNodeConfig();
  }, [questerClient]);

  const renderStorageCards = () => {
    if (nodeConfig === null) {
      return <Loader />;
    }

    const { storage_configs: storageConfigs } = nodeConfig;

    return (
      <Box
        display="flex"
        flexWrap="wrap"
        justifyContent="center" // Center the cards horizontally
        alignItems="center" // Center the cards vertically
        height="100vh" // Set the height to 100% of the viewport height
        textAlign="center" // Center the cards horizontally within the Box
      >
        {Object.entries(storageConfigs).map(([storageName, storageInfo]) => (
          <StyledCard key={storageName} sx={{ marginRight: 2, marginBottom: 2 }}>
            <CardContent>
              <Typography variant="h6">{storageName}</Typography>
              <Typography variant="body1" sx={{ marginTop: 1 }}>
                Storage Type: {storageInfo.storage_type}
              </Typography>
              <Typography variant="body1" sx={{ whiteSpace: 'pre-wrap', marginTop: 1 }}>
                Configuration:
                {JSON.stringify(storageInfo.config, null, 2)}
              </Typography>
            </CardContent>
          </StyledCard>
        ))}
      </Box>
    );
  };

  return <>{renderStorageCards()}</>;
}

export default StorageView;
