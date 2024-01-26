import { useEffect, useState } from 'react';
import {
  Box,
  Paper,
  Table,
  TableContainer,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
  Typography,
} from '@mui/material';
import { Client } from '../services/client';
import { IndexingStatistics, ResponseError } from '../utils/models';
import Loader from './Loader';

const PipelineMetadataTable = ({ pipelineId }: { pipelineId: string }) => {
  const [loading, setLoading] = useState(false);
  const [responseError, setResponseError] = useState<ResponseError | null>(null);
  const [indexingStatistics, setIndexingStatistics] = useState<IndexingStatistics | undefined>(undefined);

  const questerClient = new Client();

  useEffect(() => {
    setLoading(true);

    // Fetch Pipeline Metadata
    questerClient.getPipelineDescription(pipelineId)
      .then((stats) => {
        setResponseError(null);
        setLoading(false);
        setIndexingStatistics(stats);
      })
      .catch((error) => {
        setLoading(false);
        setResponseError(error);
      });
  }, [questerClient, pipelineId]);

  const renderPipelineMetadata = () => {
    if (responseError !== null) {
      return (
        <Typography color="error" variant="body2">
          Error loading pipeline metadata: {responseError.message}
        </Typography>
      );
    }

    if (loading) {
      return <Loader />;
    }

    if (!indexingStatistics) {
      return <Typography variant="body2">No metadata available for this pipeline.</Typography>;
    }

    return (
      <TableContainer component={Paper} sx={{ marginTop: 2 }}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>Statistic</TableCell>
              <TableCell>Value</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {Object.entries(indexingStatistics).map(([key, value]) => (
              <TableRow key={key}>
                <TableCell>{key}</TableCell>
                <TableCell>{value}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    );
  };

  return (
    <Box>
      <Typography variant="h6" gutterBottom>
        Pipeline Metadata for {pipelineId}
      </Typography>
      {renderPipelineMetadata()}
    </Box>
  );
};

export default PipelineMetadataTable;
